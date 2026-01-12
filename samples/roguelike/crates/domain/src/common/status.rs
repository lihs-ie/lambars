use std::fmt;

// =============================================================================
// StatusEffectType
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusEffectType {
    Poison,
    Burn,
    Freeze,
    Stun,
    Haste,
    Shield,
    Regeneration,
}

impl StatusEffectType {
    #[must_use]
    pub const fn is_debuff(&self) -> bool {
        matches!(self, Self::Poison | Self::Burn | Self::Freeze | Self::Stun)
    }

    #[must_use]
    pub const fn is_buff(&self) -> bool {
        matches!(self, Self::Haste | Self::Shield | Self::Regeneration)
    }

    #[must_use]
    pub const fn can_stack(&self) -> bool {
        matches!(self, Self::Shield)
    }
}

impl fmt::Display for StatusEffectType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Poison => "Poison",
            Self::Burn => "Burn",
            Self::Freeze => "Freeze",
            Self::Stun => "Stun",
            Self::Haste => "Haste",
            Self::Shield => "Shield",
            Self::Regeneration => "Regeneration",
        };
        write!(formatter, "{}", name)
    }
}

// =============================================================================
// StatusEffect
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusEffect {
    effect_type: StatusEffectType,
    remaining_turns: u32,
    potency: u32,
}

impl StatusEffect {
    #[must_use]
    pub const fn new(effect_type: StatusEffectType, remaining_turns: u32, potency: u32) -> Self {
        Self {
            effect_type,
            remaining_turns,
            potency,
        }
    }

    #[must_use]
    pub const fn effect_type(&self) -> StatusEffectType {
        self.effect_type
    }

    #[must_use]
    pub const fn remaining_turns(&self) -> u32 {
        self.remaining_turns
    }

    #[must_use]
    pub const fn potency(&self) -> u32 {
        self.potency
    }

    #[must_use]
    pub const fn tick(&self) -> Option<Self> {
        if self.remaining_turns <= 1 {
            None
        } else {
            Some(Self {
                effect_type: self.effect_type,
                remaining_turns: self.remaining_turns - 1,
                potency: self.potency,
            })
        }
    }

    #[must_use]
    pub const fn is_expired(&self) -> bool {
        self.remaining_turns == 0
    }

    #[must_use]
    pub const fn with_potency(&self, potency: u32) -> Self {
        Self {
            effect_type: self.effect_type,
            remaining_turns: self.remaining_turns,
            potency,
        }
    }
}

impl fmt::Display for StatusEffect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({} turns, potency: {})",
            self.effect_type, self.remaining_turns, self.potency
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
    // StatusEffectType Tests
    // =========================================================================

    mod status_effect_type {
        use super::*;

        #[rstest]
        #[case(StatusEffectType::Poison, true)]
        #[case(StatusEffectType::Burn, true)]
        #[case(StatusEffectType::Freeze, true)]
        #[case(StatusEffectType::Stun, true)]
        #[case(StatusEffectType::Haste, false)]
        #[case(StatusEffectType::Shield, false)]
        #[case(StatusEffectType::Regeneration, false)]
        fn is_debuff(#[case] effect_type: StatusEffectType, #[case] expected: bool) {
            assert_eq!(effect_type.is_debuff(), expected);
        }

        #[rstest]
        #[case(StatusEffectType::Poison, false)]
        #[case(StatusEffectType::Burn, false)]
        #[case(StatusEffectType::Freeze, false)]
        #[case(StatusEffectType::Stun, false)]
        #[case(StatusEffectType::Haste, true)]
        #[case(StatusEffectType::Shield, true)]
        #[case(StatusEffectType::Regeneration, true)]
        fn is_buff(#[case] effect_type: StatusEffectType, #[case] expected: bool) {
            assert_eq!(effect_type.is_buff(), expected);
        }

        #[rstest]
        #[case(StatusEffectType::Poison, false)]
        #[case(StatusEffectType::Burn, false)]
        #[case(StatusEffectType::Freeze, false)]
        #[case(StatusEffectType::Stun, false)]
        #[case(StatusEffectType::Haste, false)]
        #[case(StatusEffectType::Shield, true)]
        #[case(StatusEffectType::Regeneration, false)]
        fn can_stack(#[case] effect_type: StatusEffectType, #[case] expected: bool) {
            assert_eq!(effect_type.can_stack(), expected);
        }

        #[rstest]
        fn debuff_and_buff_are_mutually_exclusive() {
            let all_types = [
                StatusEffectType::Poison,
                StatusEffectType::Burn,
                StatusEffectType::Freeze,
                StatusEffectType::Stun,
                StatusEffectType::Haste,
                StatusEffectType::Shield,
                StatusEffectType::Regeneration,
            ];

            for effect_type in all_types {
                // An effect cannot be both buff and debuff
                assert!(!(effect_type.is_buff() && effect_type.is_debuff()));
                // An effect must be either buff or debuff
                assert!(effect_type.is_buff() || effect_type.is_debuff());
            }
        }

        #[rstest]
        #[case(StatusEffectType::Poison, "Poison")]
        #[case(StatusEffectType::Burn, "Burn")]
        #[case(StatusEffectType::Freeze, "Freeze")]
        #[case(StatusEffectType::Stun, "Stun")]
        #[case(StatusEffectType::Haste, "Haste")]
        #[case(StatusEffectType::Shield, "Shield")]
        #[case(StatusEffectType::Regeneration, "Regeneration")]
        fn display_format(#[case] effect_type: StatusEffectType, #[case] expected: &str) {
            assert_eq!(format!("{}", effect_type), expected);
        }

        #[rstest]
        fn equality() {
            assert_eq!(StatusEffectType::Poison, StatusEffectType::Poison);
            assert_ne!(StatusEffectType::Poison, StatusEffectType::Burn);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let mut set = HashSet::new();
            set.insert(StatusEffectType::Poison);

            assert!(set.contains(&StatusEffectType::Poison));
            assert!(!set.contains(&StatusEffectType::Burn));
        }
    }

    // =========================================================================
    // StatusEffect Tests
    // =========================================================================

    mod status_effect {
        use super::*;

        #[rstest]
        fn new_creates_effect() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            assert_eq!(effect.effect_type(), StatusEffectType::Poison);
            assert_eq!(effect.remaining_turns(), 3);
            assert_eq!(effect.potency(), 5);
        }

        #[rstest]
        fn new_with_zero_turns() {
            let effect = StatusEffect::new(StatusEffectType::Stun, 0, 10);
            assert_eq!(effect.remaining_turns(), 0);
            assert!(effect.is_expired());
        }

        #[rstest]
        fn tick_decrements_turns() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let ticked = effect.tick().unwrap();
            assert_eq!(ticked.remaining_turns(), 2);
            assert_eq!(ticked.potency(), 5);
        }

        #[rstest]
        fn tick_expires_at_one() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 1, 5);
            let ticked = effect.tick();
            assert!(ticked.is_none());
        }

        #[rstest]
        fn tick_expires_at_zero() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 0, 5);
            let ticked = effect.tick();
            assert!(ticked.is_none());
        }

        #[rstest]
        fn is_expired_when_zero() {
            let effect = StatusEffect::new(StatusEffectType::Stun, 0, 10);
            assert!(effect.is_expired());
        }

        #[rstest]
        fn is_expired_when_positive() {
            let effect = StatusEffect::new(StatusEffectType::Stun, 1, 10);
            assert!(!effect.is_expired());
        }

        #[rstest]
        fn with_potency_changes_potency() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let modified = effect.with_potency(10);
            assert_eq!(modified.potency(), 10);
            assert_eq!(modified.remaining_turns(), 3);
            assert_eq!(modified.effect_type(), StatusEffectType::Poison);
        }

        #[rstest]
        fn display_format() {
            let effect = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            assert_eq!(format!("{}", effect), "Poison (3 turns, potency: 5)");
        }

        #[rstest]
        fn display_format_single_turn() {
            let effect = StatusEffect::new(StatusEffectType::Haste, 1, 2);
            assert_eq!(format!("{}", effect), "Haste (1 turns, potency: 2)");
        }

        #[rstest]
        fn equality() {
            let effect1 = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect2 = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect3 = StatusEffect::new(StatusEffectType::Poison, 2, 5);
            let effect4 = StatusEffect::new(StatusEffectType::Burn, 3, 5);

            assert_eq!(effect1, effect2);
            assert_ne!(effect1, effect3);
            assert_ne!(effect1, effect4);
        }

        #[rstest]
        fn clone() {
            let effect = StatusEffect::new(StatusEffectType::Shield, 5, 20);
            let cloned = effect;
            assert_eq!(effect, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let effect1 = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect2 = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let effect3 = StatusEffect::new(StatusEffectType::Poison, 2, 5);

            let mut set = HashSet::new();
            set.insert(effect1);

            assert!(set.contains(&effect2));
            assert!(!set.contains(&effect3));
        }

        #[rstest]
        fn tick_preserves_effect_type() {
            let effect = StatusEffect::new(StatusEffectType::Regeneration, 5, 10);
            let ticked = effect.tick().unwrap();
            assert_eq!(ticked.effect_type(), StatusEffectType::Regeneration);
        }
    }
}
