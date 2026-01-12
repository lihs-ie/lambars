use std::fmt;

use serde::{Deserialize, Serialize};

// =============================================================================
// EnemyType
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnemyType {
    Goblin,

    Skeleton,

    Orc,

    Slime,

    Bat,

    Spider,

    Zombie,

    Ghost,

    Minotaur,

    Dragon,
}

impl EnemyType {
    #[must_use]
    pub const fn all() -> [Self; 10] {
        [
            Self::Goblin,
            Self::Skeleton,
            Self::Orc,
            Self::Slime,
            Self::Bat,
            Self::Spider,
            Self::Zombie,
            Self::Ghost,
            Self::Minotaur,
            Self::Dragon,
        ]
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Goblin => "Goblin",
            Self::Skeleton => "Skeleton",
            Self::Orc => "Orc",
            Self::Slime => "Slime",
            Self::Bat => "Bat",
            Self::Spider => "Spider",
            Self::Zombie => "Zombie",
            Self::Ghost => "Ghost",
            Self::Minotaur => "Minotaur",
            Self::Dragon => "Dragon",
        }
    }

    #[must_use]
    pub const fn is_boss(&self) -> bool {
        matches!(self, Self::Minotaur | Self::Dragon)
    }

    #[must_use]
    pub const fn is_undead(&self) -> bool {
        matches!(self, Self::Skeleton | Self::Zombie | Self::Ghost)
    }

    #[must_use]
    pub const fn is_flying(&self) -> bool {
        matches!(self, Self::Bat | Self::Ghost | Self::Dragon)
    }

    #[must_use]
    pub const fn base_experience(&self) -> u32 {
        match self {
            Self::Slime => 5,
            Self::Bat => 8,
            Self::Goblin => 10,
            Self::Spider => 15,
            Self::Skeleton => 20,
            Self::Zombie => 25,
            Self::Orc => 35,
            Self::Ghost => 50,
            Self::Minotaur => 200,
            Self::Dragon => 500,
        }
    }
}

impl fmt::Display for EnemyType {
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
        fn all_returns_ten_types() {
            let types = EnemyType::all();
            assert_eq!(types.len(), 10);
        }

        #[rstest]
        fn all_contains_all_variants() {
            let types = EnemyType::all();
            assert!(types.contains(&EnemyType::Goblin));
            assert!(types.contains(&EnemyType::Skeleton));
            assert!(types.contains(&EnemyType::Orc));
            assert!(types.contains(&EnemyType::Slime));
            assert!(types.contains(&EnemyType::Bat));
            assert!(types.contains(&EnemyType::Spider));
            assert!(types.contains(&EnemyType::Zombie));
            assert!(types.contains(&EnemyType::Ghost));
            assert!(types.contains(&EnemyType::Minotaur));
            assert!(types.contains(&EnemyType::Dragon));
        }

        #[rstest]
        fn all_has_unique_variants() {
            let types = EnemyType::all();
            let mut unique_count = 0;
            for (index, enemy_type) in types.iter().enumerate() {
                if !types[..index].contains(enemy_type) {
                    unique_count += 1;
                }
            }
            assert_eq!(unique_count, 10);
        }
    }

    // =========================================================================
    // Name Tests
    // =========================================================================

    mod name {
        use super::*;

        #[rstest]
        #[case(EnemyType::Goblin, "Goblin")]
        #[case(EnemyType::Skeleton, "Skeleton")]
        #[case(EnemyType::Orc, "Orc")]
        #[case(EnemyType::Slime, "Slime")]
        #[case(EnemyType::Bat, "Bat")]
        #[case(EnemyType::Spider, "Spider")]
        #[case(EnemyType::Zombie, "Zombie")]
        #[case(EnemyType::Ghost, "Ghost")]
        #[case(EnemyType::Minotaur, "Minotaur")]
        #[case(EnemyType::Dragon, "Dragon")]
        fn name_returns_correct_value(#[case] enemy_type: EnemyType, #[case] expected: &str) {
            assert_eq!(enemy_type.name(), expected);
        }
    }

    // =========================================================================
    // Boss Tests
    // =========================================================================

    mod boss {
        use super::*;

        #[rstest]
        fn minotaur_is_boss() {
            assert!(EnemyType::Minotaur.is_boss());
        }

        #[rstest]
        fn dragon_is_boss() {
            assert!(EnemyType::Dragon.is_boss());
        }

        #[rstest]
        #[case(EnemyType::Goblin)]
        #[case(EnemyType::Skeleton)]
        #[case(EnemyType::Orc)]
        #[case(EnemyType::Slime)]
        #[case(EnemyType::Bat)]
        #[case(EnemyType::Spider)]
        #[case(EnemyType::Zombie)]
        #[case(EnemyType::Ghost)]
        fn non_boss_enemies(#[case] enemy_type: EnemyType) {
            assert!(!enemy_type.is_boss());
        }
    }

    // =========================================================================
    // Undead Tests
    // =========================================================================

    mod undead {
        use super::*;

        #[rstest]
        fn skeleton_is_undead() {
            assert!(EnemyType::Skeleton.is_undead());
        }

        #[rstest]
        fn zombie_is_undead() {
            assert!(EnemyType::Zombie.is_undead());
        }

        #[rstest]
        fn ghost_is_undead() {
            assert!(EnemyType::Ghost.is_undead());
        }

        #[rstest]
        #[case(EnemyType::Goblin)]
        #[case(EnemyType::Orc)]
        #[case(EnemyType::Slime)]
        #[case(EnemyType::Bat)]
        #[case(EnemyType::Spider)]
        #[case(EnemyType::Minotaur)]
        #[case(EnemyType::Dragon)]
        fn non_undead_enemies(#[case] enemy_type: EnemyType) {
            assert!(!enemy_type.is_undead());
        }
    }

    // =========================================================================
    // Flying Tests
    // =========================================================================

    mod flying {
        use super::*;

        #[rstest]
        fn bat_is_flying() {
            assert!(EnemyType::Bat.is_flying());
        }

        #[rstest]
        fn ghost_is_flying() {
            assert!(EnemyType::Ghost.is_flying());
        }

        #[rstest]
        fn dragon_is_flying() {
            assert!(EnemyType::Dragon.is_flying());
        }

        #[rstest]
        #[case(EnemyType::Goblin)]
        #[case(EnemyType::Skeleton)]
        #[case(EnemyType::Orc)]
        #[case(EnemyType::Slime)]
        #[case(EnemyType::Spider)]
        #[case(EnemyType::Zombie)]
        #[case(EnemyType::Minotaur)]
        fn non_flying_enemies(#[case] enemy_type: EnemyType) {
            assert!(!enemy_type.is_flying());
        }
    }

    // =========================================================================
    // Base Experience Tests
    // =========================================================================

    mod base_experience {
        use super::*;

        #[rstest]
        #[case(EnemyType::Slime, 5)]
        #[case(EnemyType::Bat, 8)]
        #[case(EnemyType::Goblin, 10)]
        #[case(EnemyType::Spider, 15)]
        #[case(EnemyType::Skeleton, 20)]
        #[case(EnemyType::Zombie, 25)]
        #[case(EnemyType::Orc, 35)]
        #[case(EnemyType::Ghost, 50)]
        #[case(EnemyType::Minotaur, 200)]
        #[case(EnemyType::Dragon, 500)]
        fn base_experience_returns_correct_value(
            #[case] enemy_type: EnemyType,
            #[case] expected: u32,
        ) {
            assert_eq!(enemy_type.base_experience(), expected);
        }

        #[rstest]
        fn bosses_have_higher_experience() {
            let max_non_boss = EnemyType::all()
                .iter()
                .filter(|enemy| !enemy.is_boss())
                .map(|enemy| enemy.base_experience())
                .max()
                .unwrap();

            for enemy_type in EnemyType::all() {
                if enemy_type.is_boss() {
                    assert!(enemy_type.base_experience() > max_non_boss);
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
        #[case(EnemyType::Goblin, "Goblin")]
        #[case(EnemyType::Dragon, "Dragon")]
        fn display_format(#[case] enemy_type: EnemyType, #[case] expected: &str) {
            assert_eq!(format!("{}", enemy_type), expected);
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
            assert_eq!(EnemyType::Goblin, EnemyType::Goblin);
        }

        #[rstest]
        fn equality_different_variant() {
            assert_ne!(EnemyType::Goblin, EnemyType::Dragon);
        }

        #[rstest]
        fn hash_consistency() {
            let mut set = HashSet::new();
            set.insert(EnemyType::Goblin);

            assert!(set.contains(&EnemyType::Goblin));
            assert!(!set.contains(&EnemyType::Dragon));
        }
    }

    // =========================================================================
    // Clone and Copy Tests
    // =========================================================================

    mod clone_and_copy {
        use super::*;

        #[rstest]
        fn copy_preserves_value() {
            let enemy_type = EnemyType::Dragon;
            let copied: EnemyType = enemy_type;
            assert_eq!(enemy_type, copied);
        }
    }
}
