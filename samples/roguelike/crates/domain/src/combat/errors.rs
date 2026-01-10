//! Error types for the combat domain module.
//!
//! This module provides combat-specific error types for handling
//! combat calculation failures, targeting issues, and other
//! combat-related error conditions.

use std::error::Error;
use std::fmt;

// =============================================================================
// CombatError
// =============================================================================

/// Combat error variants for the combat domain.
///
/// This enum represents errors that can occur during combat operations
/// such as damage calculation, target validation, and turn resolution.
///
/// # Examples
///
/// ```
/// use roguelike_domain::combat::CombatError;
///
/// // Target out of range
/// let error = CombatError::TargetNotInRange {
///     attacker: (0, 0),
///     target: (10, 10),
///     range: 3,
/// };
/// assert!(error.to_string().contains("range"));
///
/// // Target not attackable
/// let error = CombatError::TargetNotAttackable {
///     target_identifier: "wall-01".to_string(),
/// };
/// assert!(error.to_string().contains("wall-01"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatError {
    /// The target is not within attack range.
    ///
    /// This error occurs when an entity attempts to attack a target
    /// that is too far away given the attack's range.
    TargetNotInRange {
        /// The position of the attacking entity.
        attacker: (i32, i32),
        /// The position of the target entity.
        target: (i32, i32),
        /// The maximum attack range.
        range: u32,
    },

    /// The target cannot be attacked.
    ///
    /// This error occurs when attempting to attack an entity that
    /// is immune to attacks or otherwise not a valid attack target
    /// (e.g., walls, friendly units, invulnerable entities).
    TargetNotAttackable {
        /// The identifier of the non-attackable target.
        target_identifier: String,
    },

    /// No valid target was found.
    ///
    /// This error occurs when an attack or ability requires a target
    /// but none is available or specified.
    NoValidTarget,

    /// The damage calculation produced an invalid result.
    ///
    /// This error occurs when the damage calculation pipeline
    /// produces an invalid or unexpected result, such as when
    /// damage modifiers conflict or produce impossible values.
    InvalidDamageCalculation,
}

impl CombatError {
    /// Returns a human-readable error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// let error = CombatError::NoValidTarget;
    /// assert_eq!(error.message(), "No valid target available for attack");
    /// ```
    pub fn message(&self) -> String {
        match self {
            Self::TargetNotInRange {
                attacker,
                target,
                range,
            } => {
                format!(
                    "Target at {:?} is not within range {} from attacker at {:?}",
                    target, range, attacker
                )
            }
            Self::TargetNotAttackable { target_identifier } => {
                format!("Target '{}' cannot be attacked", target_identifier)
            }
            Self::NoValidTarget => "No valid target available for attack".to_string(),
            Self::InvalidDamageCalculation => {
                "Damage calculation produced an invalid result".to_string()
            }
        }
    }

    /// Returns true if this error is recoverable.
    ///
    /// Recoverable errors typically indicate issues that can be
    /// resolved by the player taking different actions (e.g.,
    /// moving closer to a target or selecting a different target).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// // Player can move closer to the target
    /// let error = CombatError::TargetNotInRange {
    ///     attacker: (0, 0),
    ///     target: (10, 10),
    ///     range: 3,
    /// };
    /// assert!(error.is_recoverable());
    ///
    /// // Invalid calculation is not recoverable by player action
    /// let error = CombatError::InvalidDamageCalculation;
    /// assert!(!error.is_recoverable());
    /// ```
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::TargetNotInRange { .. } => true,
            Self::TargetNotAttackable { .. } => true,
            Self::NoValidTarget => true,
            Self::InvalidDamageCalculation => false,
        }
    }

    /// Creates a target not in range error.
    ///
    /// # Arguments
    ///
    /// * `attacker` - The position of the attacking entity
    /// * `target` - The position of the target entity
    /// * `range` - The maximum attack range
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// let error = CombatError::target_not_in_range((0, 0), (5, 5), 2);
    /// assert!(matches!(error, CombatError::TargetNotInRange { .. }));
    /// ```
    pub fn target_not_in_range(attacker: (i32, i32), target: (i32, i32), range: u32) -> Self {
        Self::TargetNotInRange {
            attacker,
            target,
            range,
        }
    }

    /// Creates a target not attackable error.
    ///
    /// # Arguments
    ///
    /// * `target_identifier` - The identifier of the non-attackable target
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// let error = CombatError::target_not_attackable("wall-01");
    /// assert!(matches!(error, CombatError::TargetNotAttackable { .. }));
    /// ```
    pub fn target_not_attackable(target_identifier: impl Into<String>) -> Self {
        Self::TargetNotAttackable {
            target_identifier: target_identifier.into(),
        }
    }

    /// Creates a no valid target error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// let error = CombatError::no_valid_target();
    /// assert!(matches!(error, CombatError::NoValidTarget));
    /// ```
    pub fn no_valid_target() -> Self {
        Self::NoValidTarget
    }

    /// Creates an invalid damage calculation error.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::combat::CombatError;
    ///
    /// let error = CombatError::invalid_damage_calculation();
    /// assert!(matches!(error, CombatError::InvalidDamageCalculation));
    /// ```
    pub fn invalid_damage_calculation() -> Self {
        Self::InvalidDamageCalculation
    }
}

impl fmt::Display for CombatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl Error for CombatError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // TargetNotInRange Tests
    // =========================================================================

    mod target_not_in_range {
        use super::*;

        #[rstest]
        fn creation_with_constructor() {
            let error = CombatError::target_not_in_range((0, 0), (5, 5), 3);

            match error {
                CombatError::TargetNotInRange {
                    attacker,
                    target,
                    range,
                } => {
                    assert_eq!(attacker, (0, 0));
                    assert_eq!(target, (5, 5));
                    assert_eq!(range, 3);
                }
                _ => panic!("Expected TargetNotInRange variant"),
            }
        }

        #[rstest]
        fn creation_with_struct_syntax() {
            let error = CombatError::TargetNotInRange {
                attacker: (1, 2),
                target: (10, 20),
                range: 5,
            };

            match error {
                CombatError::TargetNotInRange {
                    attacker,
                    target,
                    range,
                } => {
                    assert_eq!(attacker, (1, 2));
                    assert_eq!(target, (10, 20));
                    assert_eq!(range, 5);
                }
                _ => panic!("Expected TargetNotInRange variant"),
            }
        }

        #[rstest]
        fn message_contains_positions_and_range() {
            let error = CombatError::TargetNotInRange {
                attacker: (0, 0),
                target: (10, 10),
                range: 3,
            };

            let message = error.message();
            assert!(message.contains("(10, 10)"));
            assert!(message.contains("(0, 0)"));
            assert!(message.contains("3"));
        }

        #[rstest]
        fn display_matches_message() {
            let error = CombatError::TargetNotInRange {
                attacker: (0, 0),
                target: (5, 5),
                range: 2,
            };

            assert_eq!(format!("{}", error), error.message());
        }

        #[rstest]
        fn is_recoverable() {
            let error = CombatError::TargetNotInRange {
                attacker: (0, 0),
                target: (10, 10),
                range: 3,
            };

            assert!(error.is_recoverable());
        }

        #[rstest]
        fn handles_negative_coordinates() {
            let error = CombatError::target_not_in_range((-5, -3), (5, 3), 10);

            match error {
                CombatError::TargetNotInRange {
                    attacker,
                    target,
                    range,
                } => {
                    assert_eq!(attacker, (-5, -3));
                    assert_eq!(target, (5, 3));
                    assert_eq!(range, 10);
                }
                _ => panic!("Expected TargetNotInRange variant"),
            }
        }

        #[rstest]
        fn handles_zero_range() {
            let error = CombatError::target_not_in_range((0, 0), (1, 0), 0);

            match error {
                CombatError::TargetNotInRange { range, .. } => {
                    assert_eq!(range, 0);
                }
                _ => panic!("Expected TargetNotInRange variant"),
            }
        }
    }

    // =========================================================================
    // TargetNotAttackable Tests
    // =========================================================================

    mod target_not_attackable {
        use super::*;

        #[rstest]
        fn creation_with_constructor_string() {
            let error = CombatError::target_not_attackable("wall-01".to_string());

            match error {
                CombatError::TargetNotAttackable { target_identifier } => {
                    assert_eq!(target_identifier, "wall-01");
                }
                _ => panic!("Expected TargetNotAttackable variant"),
            }
        }

        #[rstest]
        fn creation_with_constructor_str() {
            let error = CombatError::target_not_attackable("friendly-npc");

            match error {
                CombatError::TargetNotAttackable { target_identifier } => {
                    assert_eq!(target_identifier, "friendly-npc");
                }
                _ => panic!("Expected TargetNotAttackable variant"),
            }
        }

        #[rstest]
        fn creation_with_struct_syntax() {
            let error = CombatError::TargetNotAttackable {
                target_identifier: "invulnerable-boss".to_string(),
            };

            match error {
                CombatError::TargetNotAttackable { target_identifier } => {
                    assert_eq!(target_identifier, "invulnerable-boss");
                }
                _ => panic!("Expected TargetNotAttackable variant"),
            }
        }

        #[rstest]
        fn message_contains_identifier() {
            let error = CombatError::TargetNotAttackable {
                target_identifier: "test-target".to_string(),
            };

            let message = error.message();
            assert!(message.contains("test-target"));
            assert!(message.contains("cannot be attacked"));
        }

        #[rstest]
        fn display_matches_message() {
            let error = CombatError::TargetNotAttackable {
                target_identifier: "wall".to_string(),
            };

            assert_eq!(format!("{}", error), error.message());
        }

        #[rstest]
        fn is_recoverable() {
            let error = CombatError::TargetNotAttackable {
                target_identifier: "wall".to_string(),
            };

            assert!(error.is_recoverable());
        }

        #[rstest]
        fn handles_empty_identifier() {
            let error = CombatError::target_not_attackable("");

            match error {
                CombatError::TargetNotAttackable { target_identifier } => {
                    assert_eq!(target_identifier, "");
                }
                _ => panic!("Expected TargetNotAttackable variant"),
            }
        }
    }

    // =========================================================================
    // NoValidTarget Tests
    // =========================================================================

    mod no_valid_target {
        use super::*;

        #[rstest]
        fn creation_with_constructor() {
            let error = CombatError::no_valid_target();
            assert!(matches!(error, CombatError::NoValidTarget));
        }

        #[rstest]
        fn creation_with_enum_syntax() {
            let error = CombatError::NoValidTarget;
            assert!(matches!(error, CombatError::NoValidTarget));
        }

        #[rstest]
        fn message_describes_error() {
            let error = CombatError::NoValidTarget;
            let message = error.message();

            assert!(message.contains("No valid target"));
        }

        #[rstest]
        fn display_matches_message() {
            let error = CombatError::NoValidTarget;
            assert_eq!(format!("{}", error), error.message());
        }

        #[rstest]
        fn is_recoverable() {
            let error = CombatError::NoValidTarget;
            assert!(error.is_recoverable());
        }
    }

    // =========================================================================
    // InvalidDamageCalculation Tests
    // =========================================================================

    mod invalid_damage_calculation {
        use super::*;

        #[rstest]
        fn creation_with_constructor() {
            let error = CombatError::invalid_damage_calculation();
            assert!(matches!(error, CombatError::InvalidDamageCalculation));
        }

        #[rstest]
        fn creation_with_enum_syntax() {
            let error = CombatError::InvalidDamageCalculation;
            assert!(matches!(error, CombatError::InvalidDamageCalculation));
        }

        #[rstest]
        fn message_describes_error() {
            let error = CombatError::InvalidDamageCalculation;
            let message = error.message();

            assert!(message.contains("invalid"));
            assert!(message.contains("Damage calculation"));
        }

        #[rstest]
        fn display_matches_message() {
            let error = CombatError::InvalidDamageCalculation;
            assert_eq!(format!("{}", error), error.message());
        }

        #[rstest]
        fn is_not_recoverable() {
            let error = CombatError::InvalidDamageCalculation;
            assert!(!error.is_recoverable());
        }
    }

    // =========================================================================
    // Common Trait Tests
    // =========================================================================

    mod common_traits {
        use super::*;

        #[rstest]
        #[case::target_not_in_range(
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 },
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 }
        )]
        #[case::target_not_attackable(
            CombatError::TargetNotAttackable { target_identifier: "test".to_string() },
            CombatError::TargetNotAttackable { target_identifier: "test".to_string() }
        )]
        #[case::no_valid_target(CombatError::NoValidTarget, CombatError::NoValidTarget)]
        #[case::invalid_damage_calculation(
            CombatError::InvalidDamageCalculation,
            CombatError::InvalidDamageCalculation
        )]
        fn equality(#[case] error1: CombatError, #[case] error2: CombatError) {
            assert_eq!(error1, error2);
        }

        #[rstest]
        #[case::different_attacker(
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 },
            CombatError::TargetNotInRange { attacker: (1, 1), target: (5, 5), range: 3 }
        )]
        #[case::different_target(
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 },
            CombatError::TargetNotInRange { attacker: (0, 0), target: (6, 6), range: 3 }
        )]
        #[case::different_range(
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 },
            CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 4 }
        )]
        #[case::different_identifier(
            CombatError::TargetNotAttackable { target_identifier: "a".to_string() },
            CombatError::TargetNotAttackable { target_identifier: "b".to_string() }
        )]
        #[case::different_variants(
            CombatError::NoValidTarget,
            CombatError::InvalidDamageCalculation
        )]
        fn inequality(#[case] error1: CombatError, #[case] error2: CombatError) {
            assert_ne!(error1, error2);
        }

        #[rstest]
        #[case::target_not_in_range(CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 })]
        #[case::target_not_attackable(CombatError::TargetNotAttackable { target_identifier: "test".to_string() })]
        #[case::no_valid_target(CombatError::NoValidTarget)]
        #[case::invalid_damage_calculation(CombatError::InvalidDamageCalculation)]
        fn clone_produces_equal_value(#[case] error: CombatError) {
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        #[case::target_not_in_range(CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 }, "TargetNotInRange")]
        #[case::target_not_attackable(CombatError::TargetNotAttackable { target_identifier: "test".to_string() }, "TargetNotAttackable")]
        #[case::no_valid_target(CombatError::NoValidTarget, "NoValidTarget")]
        #[case::invalid_damage_calculation(
            CombatError::InvalidDamageCalculation,
            "InvalidDamageCalculation"
        )]
        fn debug_contains_variant_name(#[case] error: CombatError, #[case] expected: &str) {
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains(expected));
        }

        #[rstest]
        #[case::target_not_in_range(CombatError::TargetNotInRange { attacker: (0, 0), target: (5, 5), range: 3 })]
        #[case::target_not_attackable(CombatError::TargetNotAttackable { target_identifier: "test".to_string() })]
        #[case::no_valid_target(CombatError::NoValidTarget)]
        #[case::invalid_damage_calculation(CombatError::InvalidDamageCalculation)]
        fn implements_error_trait(#[case] error: CombatError) {
            // Verify that CombatError implements std::error::Error
            let _: &dyn Error = &error;

            // source() should return None for all variants
            assert!(error.source().is_none());
        }
    }
}
