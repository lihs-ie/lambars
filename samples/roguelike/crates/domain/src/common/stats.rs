//! Composite stat types for combat and character attributes.
//!
//! This module provides compound types that combine multiple numeric values
//! for combat statistics, base attributes, and damage modification.

use std::fmt;

use lambars::typeclass::{Monoid, Semigroup};

use super::errors::ValidationError;
use super::numeric::{Attack, Damage, Defense, Health, Mana, Speed, Stat};

// =============================================================================
// CombatStats
// =============================================================================

/// Combat statistics for a character or entity.
///
/// Contains current and maximum values for health and mana,
/// as well as attack, defense, and speed values.
///
/// # Invariants
///
/// - health <= max_health
/// - mana <= max_mana
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CombatStats {
    health: Health,
    max_health: Health,
    mana: Mana,
    max_mana: Mana,
    attack: Attack,
    defense: Defense,
    speed: Speed,
}

impl CombatStats {
    /// Creates new combat stats with the given values.
    ///
    /// # Errors
    ///
    /// Returns an error if health > max_health or mana > max_mana.
    pub fn new(
        health: Health,
        max_health: Health,
        mana: Mana,
        max_mana: Mana,
        attack: Attack,
        defense: Defense,
        speed: Speed,
    ) -> Result<Self, ValidationError> {
        if health.value() > max_health.value() {
            return Err(ValidationError::constraint_violation(
                "health",
                "must not exceed max_health",
            ));
        }
        if mana.value() > max_mana.value() {
            return Err(ValidationError::constraint_violation(
                "mana",
                "must not exceed max_mana",
            ));
        }
        Ok(Self {
            health,
            max_health,
            mana,
            max_mana,
            attack,
            defense,
            speed,
        })
    }

    /// Returns the current health.
    #[must_use]
    pub const fn health(&self) -> Health {
        self.health
    }

    /// Returns the maximum health.
    #[must_use]
    pub const fn max_health(&self) -> Health {
        self.max_health
    }

    /// Returns the current mana.
    #[must_use]
    pub const fn mana(&self) -> Mana {
        self.mana
    }

    /// Returns the maximum mana.
    #[must_use]
    pub const fn max_mana(&self) -> Mana {
        self.max_mana
    }

    /// Returns the attack value.
    #[must_use]
    pub const fn attack(&self) -> Attack {
        self.attack
    }

    /// Returns the defense value.
    #[must_use]
    pub const fn defense(&self) -> Defense {
        self.defense
    }

    /// Returns the speed value.
    #[must_use]
    pub const fn speed(&self) -> Speed {
        self.speed
    }

    /// Returns a new CombatStats with the given health.
    ///
    /// # Errors
    ///
    /// Returns an error if the new health exceeds max_health.
    pub fn with_health(&self, health: Health) -> Result<Self, ValidationError> {
        Self::new(
            health,
            self.max_health,
            self.mana,
            self.max_mana,
            self.attack,
            self.defense,
            self.speed,
        )
    }

    /// Returns a new CombatStats with the given mana.
    ///
    /// # Errors
    ///
    /// Returns an error if the new mana exceeds max_mana.
    pub fn with_mana(&self, mana: Mana) -> Result<Self, ValidationError> {
        Self::new(
            self.health,
            self.max_health,
            mana,
            self.max_mana,
            self.attack,
            self.defense,
            self.speed,
        )
    }

    /// Returns true if health is greater than zero.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        !self.health.is_zero()
    }
}

// =============================================================================
// BaseStats
// =============================================================================

/// Base attribute statistics for a character.
///
/// Contains the four primary attributes: strength, dexterity,
/// intelligence, and vitality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseStats {
    strength: Stat,
    dexterity: Stat,
    intelligence: Stat,
    vitality: Stat,
}

impl BaseStats {
    /// Creates new base stats with the given values.
    #[must_use]
    pub const fn new(strength: Stat, dexterity: Stat, intelligence: Stat, vitality: Stat) -> Self {
        Self {
            strength,
            dexterity,
            intelligence,
            vitality,
        }
    }

    /// Returns the strength value.
    #[must_use]
    pub const fn strength(&self) -> Stat {
        self.strength
    }

    /// Returns the dexterity value.
    #[must_use]
    pub const fn dexterity(&self) -> Stat {
        self.dexterity
    }

    /// Returns the intelligence value.
    #[must_use]
    pub const fn intelligence(&self) -> Stat {
        self.intelligence
    }

    /// Returns the vitality value.
    #[must_use]
    pub const fn vitality(&self) -> Stat {
        self.vitality
    }

    /// Returns the sum of all stat values.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.strength.value()
            + self.dexterity.value()
            + self.intelligence.value()
            + self.vitality.value()
    }
}

// =============================================================================
// DamageModifier
// =============================================================================

/// A damage modifier that combines a multiplier and flat bonus.
///
/// When applying damage modification:
/// - First, the base damage is multiplied by the multiplier
/// - Then, the flat bonus is added
/// - The result is clamped to non-negative values
///
/// DamageModifier implements Semigroup and Monoid from lambars:
/// - Semigroup: combine multiplies multipliers and adds flat bonuses
/// - Monoid: identity is multiplier=1.0, flat_bonus=0
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::{DamageModifier, Damage};
/// use lambars::typeclass::{Semigroup, Monoid};
///
/// let modifier = DamageModifier::new(1.5, 10);
/// let base = Damage::new(100);
/// let result = modifier.apply(base);
/// assert_eq!(result.value(), 160); // 100 * 1.5 + 10 = 160
///
/// // Semigroup combination
/// let mod1 = DamageModifier::new(1.5, 10);
/// let mod2 = DamageModifier::new(2.0, 5);
/// let combined = mod1.combine(mod2);
/// assert_eq!(combined.multiplier(), 3.0);  // 1.5 * 2.0
/// assert_eq!(combined.flat_bonus(), 15);    // 10 + 5
/// ```
#[derive(Debug, Clone, Copy)]
pub struct DamageModifier {
    multiplier: f32,
    flat_bonus: i32,
}

impl DamageModifier {
    /// Creates a new DamageModifier with the given multiplier and flat bonus.
    #[must_use]
    pub const fn new(multiplier: f32, flat_bonus: i32) -> Self {
        Self {
            multiplier,
            flat_bonus,
        }
    }

    /// Returns the damage multiplier.
    #[must_use]
    pub const fn multiplier(&self) -> f32 {
        self.multiplier
    }

    /// Returns the flat damage bonus.
    #[must_use]
    pub const fn flat_bonus(&self) -> i32 {
        self.flat_bonus
    }

    /// Applies the modifier to base damage.
    ///
    /// Calculation: (base_damage * multiplier) + flat_bonus
    /// Result is clamped to 0 if negative.
    #[must_use]
    pub fn apply(&self, base_damage: Damage) -> Damage {
        let base = base_damage.value() as f32;
        let modified = (base * self.multiplier) + (self.flat_bonus as f32);
        let clamped = modified.max(0.0) as u32;
        Damage::new(clamped)
    }
}

impl PartialEq for DamageModifier {
    fn eq(&self, other: &Self) -> bool {
        const EPSILON: f32 = 1e-5;
        (self.multiplier - other.multiplier).abs() < EPSILON && self.flat_bonus == other.flat_bonus
    }
}

impl Semigroup for DamageModifier {
    /// Combines two damage modifiers.
    ///
    /// - Multipliers are multiplied together
    /// - Flat bonuses are added
    ///
    /// This operation is associative.
    fn combine(self, other: Self) -> Self {
        Self {
            multiplier: self.multiplier * other.multiplier,
            flat_bonus: self.flat_bonus + other.flat_bonus,
        }
    }
}

impl Monoid for DamageModifier {
    /// Returns the identity modifier (multiplier=1.0, flat_bonus=0).
    ///
    /// Combining any modifier with the identity returns the original modifier.
    fn empty() -> Self {
        Self {
            multiplier: 1.0,
            flat_bonus: 0,
        }
    }
}

impl fmt::Display for DamageModifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.flat_bonus >= 0 { "+" } else { "" };
        write!(
            formatter,
            "x{} {}{}",
            self.multiplier, sign, self.flat_bonus
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
    // CombatStats Tests
    // =========================================================================

    mod combat_stats {
        use super::*;

        fn create_valid_stats() -> CombatStats {
            CombatStats::new(
                Health::new(100).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            )
            .unwrap()
        }

        #[rstest]
        fn new_valid() {
            let stats = create_valid_stats();
            assert_eq!(stats.health().value(), 100);
            assert_eq!(stats.max_health().value(), 100);
            assert_eq!(stats.mana().value(), 50);
            assert_eq!(stats.max_mana().value(), 50);
            assert_eq!(stats.attack().value(), 20);
            assert_eq!(stats.defense().value(), 15);
            assert_eq!(stats.speed().value(), 10);
        }

        #[rstest]
        fn new_health_exceeds_max() {
            let result = CombatStats::new(
                Health::new(150).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            );
            assert!(result.is_err());
        }

        #[rstest]
        fn new_mana_exceeds_max() {
            let result = CombatStats::new(
                Health::new(100).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(75).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            );
            assert!(result.is_err());
        }

        #[rstest]
        fn with_health_valid() {
            let stats = create_valid_stats();
            let updated = stats.with_health(Health::new(50).unwrap()).unwrap();
            assert_eq!(updated.health().value(), 50);
        }

        #[rstest]
        fn with_health_exceeds_max() {
            let stats = create_valid_stats();
            let result = stats.with_health(Health::new(150).unwrap());
            assert!(result.is_err());
        }

        #[rstest]
        fn with_mana_valid() {
            let stats = create_valid_stats();
            let updated = stats.with_mana(Mana::new(25).unwrap()).unwrap();
            assert_eq!(updated.mana().value(), 25);
        }

        #[rstest]
        fn with_mana_exceeds_max() {
            let stats = create_valid_stats();
            let result = stats.with_mana(Mana::new(75).unwrap());
            assert!(result.is_err());
        }

        #[rstest]
        fn is_alive_when_health_positive() {
            let stats = create_valid_stats();
            assert!(stats.is_alive());
        }

        #[rstest]
        fn is_alive_when_health_zero() {
            let stats = CombatStats::new(
                Health::zero(),
                Health::new(100).unwrap(),
                Mana::new(50).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(15),
                Speed::new(10),
            )
            .unwrap();
            assert!(!stats.is_alive());
        }

        #[rstest]
        fn equality() {
            let stats1 = create_valid_stats();
            let stats2 = create_valid_stats();
            assert_eq!(stats1, stats2);
        }

        #[rstest]
        fn clone() {
            let stats = create_valid_stats();
            let cloned = stats;
            assert_eq!(stats, cloned);
        }
    }

    // =========================================================================
    // BaseStats Tests
    // =========================================================================

    mod base_stats {
        use super::*;

        fn create_base_stats() -> BaseStats {
            BaseStats::new(
                Stat::new(10).unwrap(),
                Stat::new(15).unwrap(),
                Stat::new(8).unwrap(),
                Stat::new(12).unwrap(),
            )
        }

        #[rstest]
        fn new_creates_stats() {
            let stats = create_base_stats();
            assert_eq!(stats.strength().value(), 10);
            assert_eq!(stats.dexterity().value(), 15);
            assert_eq!(stats.intelligence().value(), 8);
            assert_eq!(stats.vitality().value(), 12);
        }

        #[rstest]
        fn total_returns_sum() {
            let stats = create_base_stats();
            assert_eq!(stats.total(), 45); // 10 + 15 + 8 + 12
        }

        #[rstest]
        fn equality() {
            let stats1 = create_base_stats();
            let stats2 = create_base_stats();
            assert_eq!(stats1, stats2);
        }

        #[rstest]
        fn clone() {
            let stats = create_base_stats();
            let cloned = stats;
            assert_eq!(stats, cloned);
        }
    }

    // =========================================================================
    // DamageModifier Tests
    // =========================================================================

    mod damage_modifier {
        use super::*;

        #[rstest]
        fn new_creates_modifier() {
            let modifier = DamageModifier::new(1.5, 10);
            assert_eq!(modifier.multiplier(), 1.5);
            assert_eq!(modifier.flat_bonus(), 10);
        }

        #[rstest]
        fn apply_positive_bonus() {
            let modifier = DamageModifier::new(1.5, 10);
            let base = Damage::new(100);
            let result = modifier.apply(base);
            assert_eq!(result.value(), 160); // 100 * 1.5 + 10
        }

        #[rstest]
        fn apply_negative_bonus() {
            let modifier = DamageModifier::new(1.0, -20);
            let base = Damage::new(100);
            let result = modifier.apply(base);
            assert_eq!(result.value(), 80); // 100 * 1.0 - 20
        }

        #[rstest]
        fn apply_clamps_to_zero() {
            let modifier = DamageModifier::new(0.5, -100);
            let base = Damage::new(50);
            let result = modifier.apply(base);
            assert_eq!(result.value(), 0); // 50 * 0.5 - 100 = -75 -> 0
        }

        #[rstest]
        fn apply_with_zero_multiplier() {
            let modifier = DamageModifier::new(0.0, 10);
            let base = Damage::new(100);
            let result = modifier.apply(base);
            assert_eq!(result.value(), 10); // 100 * 0 + 10
        }

        #[rstest]
        fn display_positive_bonus() {
            let modifier = DamageModifier::new(1.5, 10);
            assert_eq!(format!("{}", modifier), "x1.5 +10");
        }

        #[rstest]
        fn display_negative_bonus() {
            let modifier = DamageModifier::new(0.8, -5);
            assert_eq!(format!("{}", modifier), "x0.8 -5");
        }

        #[rstest]
        fn equality_same_values() {
            let mod1 = DamageModifier::new(1.5, 10);
            let mod2 = DamageModifier::new(1.5, 10);
            assert_eq!(mod1, mod2);
        }

        #[rstest]
        fn equality_different_multiplier() {
            let mod1 = DamageModifier::new(1.5, 10);
            let mod2 = DamageModifier::new(2.0, 10);
            assert_ne!(mod1, mod2);
        }

        #[rstest]
        fn equality_different_bonus() {
            let mod1 = DamageModifier::new(1.5, 10);
            let mod2 = DamageModifier::new(1.5, 20);
            assert_ne!(mod1, mod2);
        }

        #[rstest]
        fn clone() {
            let modifier = DamageModifier::new(1.5, 10);
            let cloned = modifier;
            assert_eq!(modifier.multiplier(), cloned.multiplier());
            assert_eq!(modifier.flat_bonus(), cloned.flat_bonus());
        }
    }

    // =========================================================================
    // Semigroup Tests
    // =========================================================================

    mod semigroup {
        use super::*;

        #[rstest]
        fn combine_multiplies_multipliers() {
            let mod1 = DamageModifier::new(1.5, 0);
            let mod2 = DamageModifier::new(2.0, 0);
            let combined = mod1.combine(mod2);
            assert!((combined.multiplier() - 3.0).abs() < 1e-5);
        }

        #[rstest]
        fn combine_adds_flat_bonuses() {
            let mod1 = DamageModifier::new(1.0, 10);
            let mod2 = DamageModifier::new(1.0, 5);
            let combined = mod1.combine(mod2);
            assert_eq!(combined.flat_bonus(), 15);
        }

        #[rstest]
        fn combine_both_components() {
            let mod1 = DamageModifier::new(1.5, 10);
            let mod2 = DamageModifier::new(2.0, 5);
            let combined = mod1.combine(mod2);
            assert!((combined.multiplier() - 3.0).abs() < 1e-5);
            assert_eq!(combined.flat_bonus(), 15);
        }

        #[rstest]
        fn associativity() {
            let a = DamageModifier::new(1.5, 10);
            let b = DamageModifier::new(2.0, 5);
            let c = DamageModifier::new(0.5, -3);

            let left = a.combine(b).combine(c);
            let right = a.combine(b.combine(c));

            assert!((left.multiplier() - right.multiplier()).abs() < 1e-5);
            assert_eq!(left.flat_bonus(), right.flat_bonus());
        }
    }

    // =========================================================================
    // Monoid Tests
    // =========================================================================

    mod monoid {
        use super::*;

        #[rstest]
        fn empty_returns_identity() {
            let empty = DamageModifier::empty();
            assert!((empty.multiplier() - 1.0).abs() < 1e-5);
            assert_eq!(empty.flat_bonus(), 0);
        }

        #[rstest]
        fn left_identity() {
            let modifier = DamageModifier::new(1.5, 10);
            let result = DamageModifier::empty().combine(modifier);

            assert!((result.multiplier() - modifier.multiplier()).abs() < 1e-5);
            assert_eq!(result.flat_bonus(), modifier.flat_bonus());
        }

        #[rstest]
        fn right_identity() {
            let modifier = DamageModifier::new(1.5, 10);
            let result = modifier.combine(DamageModifier::empty());

            assert!((result.multiplier() - modifier.multiplier()).abs() < 1e-5);
            assert_eq!(result.flat_bonus(), modifier.flat_bonus());
        }

        #[rstest]
        fn combine_all_empty() {
            let modifiers: Vec<DamageModifier> = vec![];
            let result = DamageModifier::combine_all(modifiers);

            assert!((result.multiplier() - 1.0).abs() < 1e-5);
            assert_eq!(result.flat_bonus(), 0);
        }

        #[rstest]
        fn combine_all_multiple() {
            let modifiers = vec![
                DamageModifier::new(1.5, 10),
                DamageModifier::new(2.0, 5),
                DamageModifier::new(0.5, -3),
            ];
            let result = DamageModifier::combine_all(modifiers);

            // 1.5 * 2.0 * 0.5 = 1.5
            assert!((result.multiplier() - 1.5).abs() < 1e-5);
            // 10 + 5 - 3 = 12
            assert_eq!(result.flat_bonus(), 12);
        }
    }
}

// =============================================================================
// Property-Based Tests for Semigroup/Monoid Laws
// =============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use lambars::typeclass::{Monoid, Semigroup};
    use proptest::prelude::*;

    proptest! {
        /// Semigroup associativity law:
        /// (a.combine(b)).combine(c) == a.combine(b.combine(c))
        #[test]
        fn prop_damage_modifier_associativity(
            a_mult in -10.0f32..10.0f32,
            a_bonus in -1000i32..1000i32,
            b_mult in -10.0f32..10.0f32,
            b_bonus in -1000i32..1000i32,
            c_mult in -10.0f32..10.0f32,
            c_bonus in -1000i32..1000i32
        ) {
            let a = DamageModifier::new(a_mult, a_bonus);
            let b = DamageModifier::new(b_mult, b_bonus);
            let c = DamageModifier::new(c_mult, c_bonus);

            let left = a.combine(b).combine(c);
            let right = a.combine(b.combine(c));

            // f32 precision tolerance
            prop_assert!((left.multiplier() - right.multiplier()).abs() < 1e-4);
            prop_assert_eq!(left.flat_bonus(), right.flat_bonus());
        }

        /// Monoid left identity law:
        /// DamageModifier::empty().combine(a) == a
        #[test]
        fn prop_damage_modifier_left_identity(
            mult in -10.0f32..10.0f32,
            bonus in -1000i32..1000i32
        ) {
            let a = DamageModifier::new(mult, bonus);
            let result = DamageModifier::empty().combine(a);

            prop_assert!((result.multiplier() - a.multiplier()).abs() < 1e-5);
            prop_assert_eq!(result.flat_bonus(), a.flat_bonus());
        }

        /// Monoid right identity law:
        /// a.combine(DamageModifier::empty()) == a
        #[test]
        fn prop_damage_modifier_right_identity(
            mult in -10.0f32..10.0f32,
            bonus in -1000i32..1000i32
        ) {
            let a = DamageModifier::new(mult, bonus);
            let result = a.combine(DamageModifier::empty());

            prop_assert!((result.multiplier() - a.multiplier()).abs() < 1e-5);
            prop_assert_eq!(result.flat_bonus(), a.flat_bonus());
        }
    }
}
