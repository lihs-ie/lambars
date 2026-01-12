//! Enemy aggregate for the enemy domain.
//!
//! This module provides the Enemy aggregate root that represents
//! an enemy entity in the game world with all its associated state.

use crate::common::{CombatStats, Damage, Health, Position, StatusEffect};

use super::behavior::AiBehavior;
use super::enemy_type::EnemyType;
use super::identifier::EntityIdentifier;
use super::loot::LootTable;

// =============================================================================
// Enemy
// =============================================================================

/// An enemy entity in the game world.
///
/// Enemy is an aggregate root that encapsulates all enemy-related state
/// including position, combat statistics, behavior patterns, and status effects.
///
/// All operations on Enemy are pure functions that return new instances,
/// following functional programming principles.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
/// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
///
/// let identifier = EntityIdentifier::new();
/// let position = Position::new(5, 10);
/// let stats = CombatStats::new(
///     Health::new(100).unwrap(),
///     Health::new(100).unwrap(),
///     Mana::new(0).unwrap(),
///     Mana::new(0).unwrap(),
///     Attack::new(15),
///     Defense::new(5),
///     Speed::new(10),
/// ).unwrap();
///
/// let enemy = Enemy::new(
///     identifier,
///     EnemyType::Goblin,
///     position,
///     stats,
///     AiBehavior::Aggressive,
///     LootTable::empty(),
/// );
///
/// assert!(enemy.is_alive());
/// assert_eq!(enemy.position(), &Position::new(5, 10));
/// ```
#[derive(Debug, Clone)]
pub struct Enemy {
    identifier: EntityIdentifier,
    enemy_type: EnemyType,
    position: Position,
    stats: CombatStats,
    behavior: AiBehavior,
    loot_table: LootTable,
    status_effects: Vec<StatusEffect>,
}

impl Enemy {
    // =========================================================================
    // Constructors
    // =========================================================================

    /// Creates a new Enemy with the specified parameters.
    ///
    /// The enemy starts with no status effects applied.
    ///
    /// # Arguments
    ///
    /// * `identifier` - Unique identifier for this enemy
    /// * `enemy_type` - The type of enemy (Goblin, Skeleton, etc.)
    /// * `position` - Initial position in the game world
    /// * `stats` - Combat statistics
    /// * `behavior` - AI behavior pattern
    /// * `loot_table` - Potential item drops
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Skeleton,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(50).unwrap(),
    ///         Health::new(50).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(3),
    ///         Speed::new(8),
    ///     ).unwrap(),
    ///     AiBehavior::Patrol,
    ///     LootTable::empty(),
    /// );
    /// ```
    #[must_use]
    pub fn new(
        identifier: EntityIdentifier,
        enemy_type: EnemyType,
        position: Position,
        stats: CombatStats,
        behavior: AiBehavior,
        loot_table: LootTable,
    ) -> Self {
        Self {
            identifier,
            enemy_type,
            position,
            stats,
            behavior,
            loot_table,
            status_effects: Vec::new(),
        }
    }

    // =========================================================================
    // Getters
    // =========================================================================

    /// Returns the unique identifier for this enemy.
    #[must_use]
    pub const fn identifier(&self) -> &EntityIdentifier {
        &self.identifier
    }

    /// Returns the type of this enemy.
    #[must_use]
    pub const fn enemy_type(&self) -> &EnemyType {
        &self.enemy_type
    }

    /// Returns the current position of this enemy.
    #[must_use]
    pub const fn position(&self) -> &Position {
        &self.position
    }

    /// Returns the combat statistics of this enemy.
    #[must_use]
    pub const fn stats(&self) -> &CombatStats {
        &self.stats
    }

    /// Returns the AI behavior pattern of this enemy.
    #[must_use]
    pub const fn behavior(&self) -> &AiBehavior {
        &self.behavior
    }

    /// Returns the loot table for this enemy.
    #[must_use]
    pub const fn loot_table(&self) -> &LootTable {
        &self.loot_table
    }

    /// Returns the active status effects on this enemy.
    #[must_use]
    pub fn status_effects(&self) -> &[StatusEffect] {
        &self.status_effects
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Returns true if the enemy is alive (health > 0).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// assert!(enemy.is_alive());
    /// ```
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.stats.is_alive()
    }

    /// Returns the current health of this enemy.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(75).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// assert_eq!(enemy.health().value(), 75);
    /// ```
    #[must_use]
    pub fn health(&self) -> Health {
        self.stats.health()
    }

    // =========================================================================
    // Domain Methods (Pure Functions)
    // =========================================================================

    /// Moves the enemy to a new position.
    ///
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Arguments
    ///
    /// * `new_position` - The target position
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(5, 5),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let moved_enemy = enemy.move_to(Position::new(6, 5));
    /// assert_eq!(moved_enemy.position(), &Position::new(6, 5));
    /// ```
    #[must_use]
    pub fn move_to(self, new_position: Position) -> Self {
        Self {
            position: new_position,
            ..self
        }
    }

    /// Applies damage to the enemy.
    ///
    /// Health is reduced by the damage amount, saturating at 0.
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Arguments
    ///
    /// * `damage` - The amount of damage to apply
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed, Damage};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let damaged_enemy = enemy.take_damage(Damage::new(30));
    /// assert_eq!(damaged_enemy.health().value(), 70);
    /// ```
    #[must_use]
    pub fn take_damage(self, damage: Damage) -> Self {
        let new_health = self.stats.health().saturating_sub(damage.value());
        let new_stats = self
            .stats
            .with_health(new_health)
            .expect("health should not exceed max_health when reducing");

        Self {
            stats: new_stats,
            ..self
        }
    }

    /// Heals the enemy by the specified amount.
    ///
    /// Health is increased by the amount, saturating at max_health.
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount of health to restore
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed, Damage};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(50).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let healed_enemy = enemy.heal(30);
    /// assert_eq!(healed_enemy.health().value(), 80);
    /// ```
    #[must_use]
    pub fn heal(self, amount: u32) -> Self {
        let new_health = self.stats.health().saturating_add(amount);
        // saturating_add caps at MAX_HEALTH, but we also need to cap at max_health
        let capped_health = if new_health.value() > self.stats.max_health().value() {
            self.stats.max_health()
        } else {
            new_health
        };
        let new_stats = self
            .stats
            .with_health(capped_health)
            .expect("capped health should not exceed max_health");

        Self {
            stats: new_stats,
            ..self
        }
    }

    /// Applies a status effect to the enemy.
    ///
    /// The effect is added to the enemy's active status effects.
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Arguments
    ///
    /// * `effect` - The status effect to apply
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{
    ///     Position, CombatStats, Health, Mana, Attack, Defense, Speed,
    ///     StatusEffect, StatusEffectType
    /// };
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);
    /// let poisoned_enemy = enemy.apply_status_effect(poison);
    /// assert_eq!(poisoned_enemy.status_effects().len(), 1);
    /// ```
    #[must_use]
    pub fn apply_status_effect(self, effect: StatusEffect) -> Self {
        let mut new_effects = self.status_effects;
        new_effects.push(effect);
        Self {
            status_effects: new_effects,
            ..self
        }
    }

    /// Ticks all status effects, removing expired ones.
    ///
    /// Each effect's duration is decreased by one turn.
    /// Effects that expire (remaining_turns reaches 0) are removed.
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{
    ///     Position, CombatStats, Health, Mana, Attack, Defense, Speed,
    ///     StatusEffect, StatusEffectType
    /// };
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// let poison = StatusEffect::new(StatusEffectType::Poison, 2, 5);
    /// let enemy_with_effect = enemy.apply_status_effect(poison);
    ///
    /// // First tick: effect remains with 1 turn left
    /// let after_first_tick = enemy_with_effect.tick_status_effects();
    /// assert_eq!(after_first_tick.status_effects().len(), 1);
    /// assert_eq!(after_first_tick.status_effects()[0].remaining_turns(), 1);
    ///
    /// // Second tick: effect expires
    /// let after_second_tick = after_first_tick.tick_status_effects();
    /// assert_eq!(after_second_tick.status_effects().len(), 0);
    /// ```
    #[must_use]
    pub fn tick_status_effects(self) -> Self {
        let new_effects = self
            .status_effects
            .into_iter()
            .filter_map(|effect| effect.tick())
            .collect::<Vec<_>>();

        Self {
            status_effects: new_effects,
            ..self
        }
    }

    /// Returns a new Enemy with a different behavior pattern.
    ///
    /// This is a pure function that returns a new Enemy instance.
    ///
    /// # Arguments
    ///
    /// * `behavior` - The new AI behavior pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{Enemy, EntityIdentifier, EnemyType, AiBehavior, LootTable};
    /// use roguelike_domain::common::{Position, CombatStats, Health, Mana, Attack, Defense, Speed};
    ///
    /// let enemy = Enemy::new(
    ///     EntityIdentifier::new(),
    ///     EnemyType::Goblin,
    ///     Position::new(0, 0),
    ///     CombatStats::new(
    ///         Health::new(100).unwrap(),
    ///         Health::new(100).unwrap(),
    ///         Mana::zero(),
    ///         Mana::zero(),
    ///         Attack::new(10),
    ///         Defense::new(5),
    ///         Speed::new(10),
    ///     ).unwrap(),
    ///     AiBehavior::Aggressive,
    ///     LootTable::empty(),
    /// );
    ///
    /// // Enemy flees when health is low
    /// let fleeing_enemy = enemy.with_behavior(AiBehavior::Flee);
    /// assert_eq!(fleeing_enemy.behavior(), &AiBehavior::Flee);
    /// ```
    #[must_use]
    pub fn with_behavior(self, behavior: AiBehavior) -> Self {
        Self { behavior, ..self }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{Attack, Defense, Mana, Speed, StatusEffectType};
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_test_enemy() -> Enemy {
        Enemy::new(
            EntityIdentifier::new(),
            EnemyType::Goblin,
            Position::new(5, 5),
            CombatStats::new(
                Health::new(100).unwrap(),
                Health::new(100).unwrap(),
                Mana::zero(),
                Mana::zero(),
                Attack::new(15),
                Defense::new(5),
                Speed::new(10),
            )
            .unwrap(),
            AiBehavior::Aggressive,
            LootTable::empty(),
        )
    }

    fn create_test_enemy_with_health(current: u32, max: u32) -> Enemy {
        Enemy::new(
            EntityIdentifier::new(),
            EnemyType::Skeleton,
            Position::new(0, 0),
            CombatStats::new(
                Health::new(current).unwrap(),
                Health::new(max).unwrap(),
                Mana::zero(),
                Mana::zero(),
                Attack::new(10),
                Defense::new(3),
                Speed::new(8),
            )
            .unwrap(),
            AiBehavior::Patrol,
            LootTable::empty(),
        )
    }

    // =========================================================================
    // Construction Tests
    // =========================================================================

    mod construction {
        use super::*;

        #[rstest]
        fn new_creates_enemy_with_correct_values() {
            let identifier = EntityIdentifier::new();
            let position = Position::new(10, 20);
            let stats = CombatStats::new(
                Health::new(50).unwrap(),
                Health::new(100).unwrap(),
                Mana::new(25).unwrap(),
                Mana::new(50).unwrap(),
                Attack::new(20),
                Defense::new(10),
                Speed::new(15),
            )
            .unwrap();

            let enemy = Enemy::new(
                identifier,
                EnemyType::Dragon,
                position,
                stats,
                AiBehavior::Aggressive,
                LootTable::empty(),
            );

            assert_eq!(*enemy.identifier(), identifier);
            assert_eq!(*enemy.enemy_type(), EnemyType::Dragon);
            assert_eq!(*enemy.position(), Position::new(10, 20));
            assert_eq!(*enemy.behavior(), AiBehavior::Aggressive);
            assert!(enemy.status_effects().is_empty());
        }

        #[rstest]
        fn new_creates_enemy_with_no_status_effects() {
            let enemy = create_test_enemy();
            assert!(enemy.status_effects().is_empty());
        }
    }

    // =========================================================================
    // Getter Tests
    // =========================================================================

    mod getters {
        use super::*;

        #[rstest]
        fn identifier_returns_correct_value() {
            let identifier = EntityIdentifier::new();
            let enemy = Enemy::new(
                identifier,
                EnemyType::Goblin,
                Position::new(0, 0),
                CombatStats::new(
                    Health::new(100).unwrap(),
                    Health::new(100).unwrap(),
                    Mana::zero(),
                    Mana::zero(),
                    Attack::new(10),
                    Defense::new(5),
                    Speed::new(10),
                )
                .unwrap(),
                AiBehavior::Aggressive,
                LootTable::empty(),
            );

            assert_eq!(*enemy.identifier(), identifier);
        }

        #[rstest]
        fn enemy_type_returns_correct_value() {
            let enemy = create_test_enemy();
            assert_eq!(*enemy.enemy_type(), EnemyType::Goblin);
        }

        #[rstest]
        fn position_returns_correct_value() {
            let enemy = create_test_enemy();
            assert_eq!(*enemy.position(), Position::new(5, 5));
        }

        #[rstest]
        fn stats_returns_correct_value() {
            let enemy = create_test_enemy();
            assert_eq!(enemy.stats().health().value(), 100);
            assert_eq!(enemy.stats().attack().value(), 15);
        }

        #[rstest]
        fn behavior_returns_correct_value() {
            let enemy = create_test_enemy();
            assert_eq!(*enemy.behavior(), AiBehavior::Aggressive);
        }

        #[rstest]
        fn loot_table_returns_correct_value() {
            let enemy = create_test_enemy();
            assert!(enemy.loot_table().is_empty());
        }
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    mod query_methods {
        use super::*;

        #[rstest]
        fn is_alive_when_health_positive() {
            let enemy = create_test_enemy();
            assert!(enemy.is_alive());
        }

        #[rstest]
        fn is_alive_when_health_zero() {
            let enemy = create_test_enemy_with_health(0, 100);
            assert!(!enemy.is_alive());
        }

        #[rstest]
        fn is_alive_when_health_minimal() {
            let enemy = create_test_enemy_with_health(1, 100);
            assert!(enemy.is_alive());
        }

        #[rstest]
        fn health_returns_current_health() {
            let enemy = create_test_enemy_with_health(75, 100);
            assert_eq!(enemy.health().value(), 75);
        }
    }

    // =========================================================================
    // Movement Tests
    // =========================================================================

    mod movement {
        use super::*;

        #[rstest]
        fn move_to_changes_position() {
            let enemy = create_test_enemy();
            let moved = enemy.move_to(Position::new(10, 15));
            assert_eq!(*moved.position(), Position::new(10, 15));
        }

        #[rstest]
        fn move_to_preserves_other_fields() {
            let enemy = create_test_enemy();
            let original_identifier = *enemy.identifier();
            let original_health = enemy.health().value();

            let moved = enemy.move_to(Position::new(10, 15));

            assert_eq!(*moved.identifier(), original_identifier);
            assert_eq!(moved.health().value(), original_health);
            assert_eq!(*moved.enemy_type(), EnemyType::Goblin);
        }

        #[rstest]
        fn move_to_same_position() {
            let enemy = create_test_enemy();
            let moved = enemy.move_to(Position::new(5, 5));
            assert_eq!(*moved.position(), Position::new(5, 5));
        }

        #[rstest]
        fn move_to_negative_coordinates() {
            let enemy = create_test_enemy();
            let moved = enemy.move_to(Position::new(-5, -10));
            assert_eq!(*moved.position(), Position::new(-5, -10));
        }
    }

    // =========================================================================
    // Damage Tests
    // =========================================================================

    mod damage {
        use super::*;

        #[rstest]
        fn take_damage_reduces_health() {
            let enemy = create_test_enemy();
            let damaged = enemy.take_damage(Damage::new(30));
            assert_eq!(damaged.health().value(), 70);
        }

        #[rstest]
        fn take_damage_saturates_at_zero() {
            let enemy = create_test_enemy();
            let damaged = enemy.take_damage(Damage::new(150));
            assert_eq!(damaged.health().value(), 0);
            assert!(!damaged.is_alive());
        }

        #[rstest]
        fn take_damage_exact_health() {
            let enemy = create_test_enemy();
            let damaged = enemy.take_damage(Damage::new(100));
            assert_eq!(damaged.health().value(), 0);
        }

        #[rstest]
        fn take_damage_zero_damage() {
            let enemy = create_test_enemy();
            let damaged = enemy.take_damage(Damage::new(0));
            assert_eq!(damaged.health().value(), 100);
        }

        #[rstest]
        fn take_damage_preserves_other_fields() {
            let enemy = create_test_enemy();
            let original_position = *enemy.position();

            let damaged = enemy.take_damage(Damage::new(30));

            assert_eq!(*damaged.position(), original_position);
            assert_eq!(*damaged.enemy_type(), EnemyType::Goblin);
        }
    }

    // =========================================================================
    // Healing Tests
    // =========================================================================

    mod healing {
        use super::*;

        #[rstest]
        fn heal_increases_health() {
            let enemy = create_test_enemy_with_health(50, 100);
            let healed = enemy.heal(30);
            assert_eq!(healed.health().value(), 80);
        }

        #[rstest]
        fn heal_saturates_at_max_health() {
            let enemy = create_test_enemy_with_health(80, 100);
            let healed = enemy.heal(50);
            assert_eq!(healed.health().value(), 100);
        }

        #[rstest]
        fn heal_to_exactly_max() {
            let enemy = create_test_enemy_with_health(70, 100);
            let healed = enemy.heal(30);
            assert_eq!(healed.health().value(), 100);
        }

        #[rstest]
        fn heal_zero_amount() {
            let enemy = create_test_enemy_with_health(50, 100);
            let healed = enemy.heal(0);
            assert_eq!(healed.health().value(), 50);
        }

        #[rstest]
        fn heal_at_full_health() {
            let enemy = create_test_enemy();
            let healed = enemy.heal(50);
            assert_eq!(healed.health().value(), 100);
        }

        #[rstest]
        fn heal_preserves_other_fields() {
            let enemy = create_test_enemy_with_health(50, 100);
            let original_position = *enemy.position();

            let healed = enemy.heal(30);

            assert_eq!(*healed.position(), original_position);
            assert_eq!(*healed.enemy_type(), EnemyType::Skeleton);
        }
    }

    // =========================================================================
    // Status Effect Tests
    // =========================================================================

    mod status_effects {
        use super::*;

        #[rstest]
        fn apply_status_effect_adds_effect() {
            let enemy = create_test_enemy();
            let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);

            let affected = enemy.apply_status_effect(poison);

            assert_eq!(affected.status_effects().len(), 1);
            assert_eq!(
                affected.status_effects()[0].effect_type(),
                StatusEffectType::Poison
            );
        }

        #[rstest]
        fn apply_multiple_status_effects() {
            let enemy = create_test_enemy();
            let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let burn = StatusEffect::new(StatusEffectType::Burn, 2, 10);

            let affected = enemy.apply_status_effect(poison).apply_status_effect(burn);

            assert_eq!(affected.status_effects().len(), 2);
        }

        #[rstest]
        fn tick_status_effects_decrements_duration() {
            let enemy = create_test_enemy();
            let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);

            let affected = enemy.apply_status_effect(poison);
            let ticked = affected.tick_status_effects();

            assert_eq!(ticked.status_effects().len(), 1);
            assert_eq!(ticked.status_effects()[0].remaining_turns(), 2);
        }

        #[rstest]
        fn tick_status_effects_removes_expired() {
            let enemy = create_test_enemy();
            let poison = StatusEffect::new(StatusEffectType::Poison, 1, 5);

            let affected = enemy.apply_status_effect(poison);
            let ticked = affected.tick_status_effects();

            assert!(ticked.status_effects().is_empty());
        }

        #[rstest]
        fn tick_status_effects_mixed_durations() {
            let enemy = create_test_enemy();
            let short_effect = StatusEffect::new(StatusEffectType::Poison, 1, 5);
            let long_effect = StatusEffect::new(StatusEffectType::Burn, 5, 10);

            let affected = enemy
                .apply_status_effect(short_effect)
                .apply_status_effect(long_effect);

            let ticked = affected.tick_status_effects();

            assert_eq!(ticked.status_effects().len(), 1);
            assert_eq!(
                ticked.status_effects()[0].effect_type(),
                StatusEffectType::Burn
            );
            assert_eq!(ticked.status_effects()[0].remaining_turns(), 4);
        }

        #[rstest]
        fn tick_empty_status_effects() {
            let enemy = create_test_enemy();
            let ticked = enemy.tick_status_effects();
            assert!(ticked.status_effects().is_empty());
        }
    }

    // =========================================================================
    // Behavior Tests
    // =========================================================================

    mod behavior_change {
        use super::*;

        #[rstest]
        fn with_behavior_changes_behavior() {
            let enemy = create_test_enemy();
            let fleeing = enemy.with_behavior(AiBehavior::Flee);
            assert_eq!(*fleeing.behavior(), AiBehavior::Flee);
        }

        #[rstest]
        fn with_behavior_preserves_other_fields() {
            let enemy = create_test_enemy();
            let original_health = enemy.health().value();
            let original_position = *enemy.position();

            let changed = enemy.with_behavior(AiBehavior::Defensive);

            assert_eq!(changed.health().value(), original_health);
            assert_eq!(*changed.position(), original_position);
        }

        #[rstest]
        #[case(AiBehavior::Aggressive)]
        #[case(AiBehavior::Defensive)]
        #[case(AiBehavior::Passive)]
        #[case(AiBehavior::Patrol)]
        #[case(AiBehavior::Flee)]
        fn with_behavior_all_variants(#[case] behavior: AiBehavior) {
            let enemy = create_test_enemy();
            let changed = enemy.with_behavior(behavior);
            assert_eq!(*changed.behavior(), behavior);
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    mod clone {
        use super::*;

        #[rstest]
        fn clone_creates_independent_copy() {
            let enemy = create_test_enemy();
            let cloned = enemy.clone();

            assert_eq!(*cloned.identifier(), *enemy.identifier());
            assert_eq!(*cloned.position(), *enemy.position());
            assert_eq!(cloned.health().value(), enemy.health().value());
        }

        #[rstest]
        fn clone_with_status_effects() {
            let enemy = create_test_enemy();
            let poison = StatusEffect::new(StatusEffectType::Poison, 3, 5);
            let affected = enemy.apply_status_effect(poison);

            let cloned = affected.clone();

            assert_eq!(cloned.status_effects().len(), 1);
            assert_eq!(affected.status_effects(), cloned.status_effects());
        }
    }

    // =========================================================================
    // Immutability Tests
    // =========================================================================

    mod immutability {
        use super::*;

        #[rstest]
        fn move_to_does_not_modify_original() {
            let original = create_test_enemy();
            let original_position = *original.position();

            let _moved = original.clone().move_to(Position::new(100, 100));

            assert_eq!(*original.position(), original_position);
        }

        #[rstest]
        fn take_damage_does_not_modify_original() {
            let original = create_test_enemy();
            let original_health = original.health().value();

            let _damaged = original.clone().take_damage(Damage::new(50));

            assert_eq!(original.health().value(), original_health);
        }

        #[rstest]
        fn heal_does_not_modify_original() {
            let original = create_test_enemy_with_health(50, 100);
            let original_health = original.health().value();

            let _healed = original.clone().heal(30);

            assert_eq!(original.health().value(), original_health);
        }
    }
}
