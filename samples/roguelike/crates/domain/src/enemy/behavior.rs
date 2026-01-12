use std::fmt;

use serde::{Deserialize, Serialize};

// =============================================================================
// AiBehavior
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiBehavior {
    Aggressive,

    Defensive,

    Passive,

    Patrol,

    Flee,
}

impl AiBehavior {
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Aggressive,
            Self::Defensive,
            Self::Passive,
            Self::Patrol,
            Self::Flee,
        ]
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Aggressive => "Aggressive",
            Self::Defensive => "Defensive",
            Self::Passive => "Passive",
            Self::Patrol => "Patrol",
            Self::Flee => "Flee",
        }
    }

    #[must_use]
    pub const fn seeks_player(&self) -> bool {
        matches!(self, Self::Aggressive)
    }

    #[must_use]
    pub const fn avoids_player(&self) -> bool {
        matches!(self, Self::Flee)
    }

    #[must_use]
    pub const fn initiates_combat(&self) -> bool {
        matches!(self, Self::Aggressive | Self::Patrol)
    }

    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Aggressive => 5,
            Self::Defensive => 3,
            Self::Patrol => 2,
            Self::Passive => 1,
            Self::Flee => 0,
        }
    }
}

impl fmt::Display for AiBehavior {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.name())
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
    // All Variants Tests
    // =========================================================================

    mod all_variants {
        use super::*;

        #[rstest]
        fn all_returns_five_behaviors() {
            let behaviors = AiBehavior::all();
            assert_eq!(behaviors.len(), 5);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let behaviors = AiBehavior::all();
            assert!(behaviors.contains(&AiBehavior::Aggressive));
            assert!(behaviors.contains(&AiBehavior::Defensive));
            assert!(behaviors.contains(&AiBehavior::Passive));
            assert!(behaviors.contains(&AiBehavior::Patrol));
            assert!(behaviors.contains(&AiBehavior::Flee));
        }

        #[rstest]
        fn all_has_unique_variants() {
            let behaviors = AiBehavior::all();
            let mut unique_count = 0;
            for (index, behavior) in behaviors.iter().enumerate() {
                if !behaviors[..index].contains(behavior) {
                    unique_count += 1;
                }
            }
            assert_eq!(unique_count, 5);
        }
    }

    // =========================================================================
    // Name Tests
    // =========================================================================

    mod name {
        use super::*;

        #[rstest]
        #[case(AiBehavior::Aggressive, "Aggressive")]
        #[case(AiBehavior::Defensive, "Defensive")]
        #[case(AiBehavior::Passive, "Passive")]
        #[case(AiBehavior::Patrol, "Patrol")]
        #[case(AiBehavior::Flee, "Flee")]
        fn name_returns_correct_value(#[case] behavior: AiBehavior, #[case] expected: &str) {
            assert_eq!(behavior.name(), expected);
        }
    }

    // =========================================================================
    // Seeks Player Tests
    // =========================================================================

    mod seeks_player {
        use super::*;

        #[rstest]
        fn aggressive_seeks_player() {
            assert!(AiBehavior::Aggressive.seeks_player());
        }

        #[rstest]
        #[case(AiBehavior::Defensive)]
        #[case(AiBehavior::Passive)]
        #[case(AiBehavior::Patrol)]
        #[case(AiBehavior::Flee)]
        fn non_seeking_behaviors(#[case] behavior: AiBehavior) {
            assert!(!behavior.seeks_player());
        }
    }

    // =========================================================================
    // Avoids Player Tests
    // =========================================================================

    mod avoids_player {
        use super::*;

        #[rstest]
        fn flee_avoids_player() {
            assert!(AiBehavior::Flee.avoids_player());
        }

        #[rstest]
        #[case(AiBehavior::Aggressive)]
        #[case(AiBehavior::Defensive)]
        #[case(AiBehavior::Passive)]
        #[case(AiBehavior::Patrol)]
        fn non_avoiding_behaviors(#[case] behavior: AiBehavior) {
            assert!(!behavior.avoids_player());
        }
    }

    // =========================================================================
    // Initiates Combat Tests
    // =========================================================================

    mod initiates_combat {
        use super::*;

        #[rstest]
        fn aggressive_initiates_combat() {
            assert!(AiBehavior::Aggressive.initiates_combat());
        }

        #[rstest]
        fn patrol_initiates_combat() {
            assert!(AiBehavior::Patrol.initiates_combat());
        }

        #[rstest]
        #[case(AiBehavior::Defensive)]
        #[case(AiBehavior::Passive)]
        #[case(AiBehavior::Flee)]
        fn non_initiating_behaviors(#[case] behavior: AiBehavior) {
            assert!(!behavior.initiates_combat());
        }
    }

    // =========================================================================
    // Priority Tests
    // =========================================================================

    mod priority {
        use super::*;

        #[rstest]
        #[case(AiBehavior::Aggressive, 5)]
        #[case(AiBehavior::Defensive, 3)]
        #[case(AiBehavior::Patrol, 2)]
        #[case(AiBehavior::Passive, 1)]
        #[case(AiBehavior::Flee, 0)]
        fn priority_returns_correct_value(#[case] behavior: AiBehavior, #[case] expected: u8) {
            assert_eq!(behavior.priority(), expected);
        }

        #[rstest]
        fn aggressive_has_highest_priority() {
            let aggressive = AiBehavior::Aggressive.priority();
            for behavior in AiBehavior::all() {
                if behavior != AiBehavior::Aggressive {
                    assert!(aggressive > behavior.priority());
                }
            }
        }

        #[rstest]
        fn flee_has_lowest_priority() {
            let flee = AiBehavior::Flee.priority();
            for behavior in AiBehavior::all() {
                if behavior != AiBehavior::Flee {
                    assert!(flee < behavior.priority());
                }
            }
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        #[case(AiBehavior::Aggressive, "Aggressive")]
        #[case(AiBehavior::Flee, "Flee")]
        fn display_format(#[case] behavior: AiBehavior, #[case] expected: &str) {
            assert_eq!(format!("{}", behavior), expected);
        }
    }

    // =========================================================================
    // Equality and Hash Tests
    // =========================================================================

    mod equality_and_hash {
        use super::*;
        use std::collections::HashSet;

        #[rstest]
        fn equality_same_variant() {
            assert_eq!(AiBehavior::Aggressive, AiBehavior::Aggressive);
        }

        #[rstest]
        fn equality_different_variant() {
            assert_ne!(AiBehavior::Aggressive, AiBehavior::Flee);
        }

        #[rstest]
        fn hash_consistency() {
            let mut set = HashSet::new();
            set.insert(AiBehavior::Aggressive);

            assert!(set.contains(&AiBehavior::Aggressive));
            assert!(!set.contains(&AiBehavior::Flee));
        }
    }

    // =========================================================================
    // Clone and Copy Tests
    // =========================================================================

    mod clone_and_copy {
        use super::*;

        #[rstest]
        fn copy_preserves_value() {
            let behavior = AiBehavior::Aggressive;
            let copied: AiBehavior = behavior;
            assert_eq!(behavior, copied);
        }
    }
}
