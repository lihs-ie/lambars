//! Domain events for the enemy domain.
//!
//! This module provides domain events that represent significant
//! occurrences in the enemy lifecycle.

use crate::common::{Damage, Position};

use super::enemy_type::EnemyType;
use super::identifier::EntityIdentifier;
use super::loot::LootTable;

// =============================================================================
// EnemySpawned
// =============================================================================

/// Event emitted when a new enemy spawns in the game world.
///
/// This event captures the initial state of an enemy when it first
/// appears on a floor.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{EnemySpawned, EntityIdentifier, EnemyType};
/// use roguelike_domain::common::Position;
///
/// let event = EnemySpawned::new(
///     EntityIdentifier::new(),
///     EnemyType::Goblin,
///     Position::new(5, 10),
/// );
/// println!("Enemy spawned: {}", event.enemy_type());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemySpawned {
    enemy_identifier: EntityIdentifier,
    enemy_type: EnemyType,
    position: Position,
}

impl EnemySpawned {
    /// Creates a new EnemySpawned event.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The unique identifier of the spawned enemy
    /// * `enemy_type` - The type of enemy that spawned
    /// * `position` - The position where the enemy spawned
    #[must_use]
    pub const fn new(
        enemy_identifier: EntityIdentifier,
        enemy_type: EnemyType,
        position: Position,
    ) -> Self {
        Self {
            enemy_identifier,
            enemy_type,
            position,
        }
    }

    /// Returns the enemy identifier.
    #[must_use]
    pub const fn enemy_identifier(&self) -> EntityIdentifier {
        self.enemy_identifier
    }

    /// Returns the enemy type.
    #[must_use]
    pub const fn enemy_type(&self) -> EnemyType {
        self.enemy_type
    }

    /// Returns the spawn position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

// =============================================================================
// EnemyMoved
// =============================================================================

/// Event emitted when an enemy moves to a new position.
///
/// This event captures both the origin and destination of a movement.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{EnemyMoved, EntityIdentifier};
/// use roguelike_domain::common::Position;
///
/// let event = EnemyMoved::new(
///     EntityIdentifier::new(),
///     Position::new(5, 10),
///     Position::new(6, 10),
/// );
/// println!("Enemy moved from {} to {}", event.from(), event.to());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyMoved {
    enemy_identifier: EntityIdentifier,
    from: Position,
    to: Position,
}

impl EnemyMoved {
    /// Creates a new EnemyMoved event.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The unique identifier of the enemy that moved
    /// * `from` - The position the enemy moved from
    /// * `to` - The position the enemy moved to
    #[must_use]
    pub const fn new(enemy_identifier: EntityIdentifier, from: Position, to: Position) -> Self {
        Self {
            enemy_identifier,
            from,
            to,
        }
    }

    /// Returns the enemy identifier.
    #[must_use]
    pub const fn enemy_identifier(&self) -> EntityIdentifier {
        self.enemy_identifier
    }

    /// Returns the origin position.
    #[must_use]
    pub const fn from(&self) -> Position {
        self.from
    }

    /// Returns the destination position.
    #[must_use]
    pub const fn to(&self) -> Position {
        self.to
    }

    /// Returns the distance moved (Manhattan distance).
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::{EnemyMoved, EntityIdentifier};
    /// use roguelike_domain::common::{Position, Distance};
    ///
    /// let event = EnemyMoved::new(
    ///     EntityIdentifier::new(),
    ///     Position::new(0, 0),
    ///     Position::new(3, 4),
    /// );
    /// assert_eq!(event.distance().value(), 7);
    /// ```
    #[must_use]
    pub fn distance(&self) -> crate::common::Distance {
        self.from.distance_to(&self.to)
    }
}

// =============================================================================
// EnemyAttacked
// =============================================================================

/// Event emitted when an enemy receives damage.
///
/// This event captures the damage dealt to an enemy without
/// specifying the source of the damage.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{EnemyAttacked, EntityIdentifier};
/// use roguelike_domain::common::Damage;
///
/// let event = EnemyAttacked::new(
///     EntityIdentifier::new(),
///     Damage::new(50),
/// );
/// println!("Enemy took {} damage", event.damage().value());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyAttacked {
    enemy_identifier: EntityIdentifier,
    damage: Damage,
}

impl EnemyAttacked {
    /// Creates a new EnemyAttacked event.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The unique identifier of the attacked enemy
    /// * `damage` - The amount of damage dealt
    #[must_use]
    pub const fn new(enemy_identifier: EntityIdentifier, damage: Damage) -> Self {
        Self {
            enemy_identifier,
            damage,
        }
    }

    /// Returns the enemy identifier.
    #[must_use]
    pub const fn enemy_identifier(&self) -> EntityIdentifier {
        self.enemy_identifier
    }

    /// Returns the damage dealt.
    #[must_use]
    pub const fn damage(&self) -> Damage {
        self.damage
    }

    /// Returns true if the damage was zero (blocked or absorbed).
    #[must_use]
    pub fn was_blocked(&self) -> bool {
        self.damage.value() == 0
    }
}

// =============================================================================
// EnemyDied
// =============================================================================

/// Event emitted when an enemy dies.
///
/// This event captures the enemy's death and the loot table that
/// defines potential item drops.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{EnemyDied, EntityIdentifier, LootTable};
///
/// let event = EnemyDied::new(
///     EntityIdentifier::new(),
///     LootTable::empty(),
/// );
/// println!("Enemy died with {} loot entries", event.loot().len());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnemyDied {
    enemy_identifier: EntityIdentifier,
    loot: LootTable,
}

impl EnemyDied {
    /// Creates a new EnemyDied event.
    ///
    /// # Arguments
    ///
    /// * `enemy_identifier` - The unique identifier of the dead enemy
    /// * `loot` - The loot table for potential drops
    #[must_use]
    pub const fn new(enemy_identifier: EntityIdentifier, loot: LootTable) -> Self {
        Self {
            enemy_identifier,
            loot,
        }
    }

    /// Returns the enemy identifier.
    #[must_use]
    pub const fn enemy_identifier(&self) -> EntityIdentifier {
        self.enemy_identifier
    }

    /// Returns the loot table.
    #[must_use]
    pub const fn loot(&self) -> &LootTable {
        &self.loot
    }

    /// Returns true if the enemy has any potential loot.
    #[must_use]
    pub fn has_loot(&self) -> bool {
        !self.loot.is_empty()
    }
}

// =============================================================================
// EnemyEvent
// =============================================================================

/// A unified enum for all enemy-related domain events.
///
/// This enum allows handling any enemy event through pattern matching.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::{
///     EnemyEvent, EnemySpawned, EntityIdentifier, EnemyType
/// };
/// use roguelike_domain::common::Position;
///
/// let spawned = EnemySpawned::new(
///     EntityIdentifier::new(),
///     EnemyType::Goblin,
///     Position::new(5, 10),
/// );
/// let event = EnemyEvent::Spawned(spawned);
///
/// match event {
///     EnemyEvent::Spawned(e) => println!("Enemy spawned: {}", e.enemy_type()),
///     _ => {}
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum EnemyEvent {
    /// An enemy has spawned.
    Spawned(EnemySpawned),

    /// An enemy has moved.
    Moved(EnemyMoved),

    /// An enemy has been attacked.
    Attacked(EnemyAttacked),

    /// An enemy has died.
    Died(EnemyDied),
}

impl EnemyEvent {
    /// Returns the enemy identifier associated with this event.
    #[must_use]
    pub const fn enemy_identifier(&self) -> EntityIdentifier {
        match self {
            Self::Spawned(event) => event.enemy_identifier(),
            Self::Moved(event) => event.enemy_identifier(),
            Self::Attacked(event) => event.enemy_identifier(),
            Self::Died(event) => event.enemy_identifier(),
        }
    }

    /// Returns true if this is a spawn event.
    #[must_use]
    pub const fn is_spawn(&self) -> bool {
        matches!(self, Self::Spawned(_))
    }

    /// Returns true if this is a movement event.
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        matches!(self, Self::Moved(_))
    }

    /// Returns true if this is an attack event.
    #[must_use]
    pub const fn is_attack(&self) -> bool {
        matches!(self, Self::Attacked(_))
    }

    /// Returns true if this is a death event.
    #[must_use]
    pub const fn is_death(&self) -> bool {
        matches!(self, Self::Died(_))
    }
}

impl From<EnemySpawned> for EnemyEvent {
    fn from(event: EnemySpawned) -> Self {
        Self::Spawned(event)
    }
}

impl From<EnemyMoved> for EnemyEvent {
    fn from(event: EnemyMoved) -> Self {
        Self::Moved(event)
    }
}

impl From<EnemyAttacked> for EnemyEvent {
    fn from(event: EnemyAttacked) -> Self {
        Self::Attacked(event)
    }
}

impl From<EnemyDied> for EnemyEvent {
    fn from(event: EnemyDied) -> Self {
        Self::Died(event)
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
    // EnemySpawned Tests
    // =========================================================================

    mod enemy_spawned {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = EntityIdentifier::new();
            let event = EnemySpawned::new(identifier, EnemyType::Goblin, Position::new(5, 10));

            assert_eq!(event.enemy_identifier(), identifier);
            assert_eq!(event.enemy_type(), EnemyType::Goblin);
            assert_eq!(event.position(), Position::new(5, 10));
        }

        #[rstest]
        fn clone_preserves_values() {
            let identifier = EntityIdentifier::new();
            let event = EnemySpawned::new(identifier, EnemyType::Dragon, Position::new(0, 0));
            let cloned = event.clone();

            assert_eq!(event, cloned);
        }

        #[rstest]
        fn equality_same_values() {
            let identifier = EntityIdentifier::new();
            let event1 = EnemySpawned::new(identifier, EnemyType::Goblin, Position::new(5, 10));
            let event2 = EnemySpawned::new(identifier, EnemyType::Goblin, Position::new(5, 10));

            assert_eq!(event1, event2);
        }

        #[rstest]
        fn inequality_different_identifier() {
            let event1 = EnemySpawned::new(
                EntityIdentifier::new(),
                EnemyType::Goblin,
                Position::new(5, 10),
            );
            let event2 = EnemySpawned::new(
                EntityIdentifier::new(),
                EnemyType::Goblin,
                Position::new(5, 10),
            );

            assert_ne!(event1, event2);
        }
    }

    // =========================================================================
    // EnemyMoved Tests
    // =========================================================================

    mod enemy_moved {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = EntityIdentifier::new();
            let event = EnemyMoved::new(identifier, Position::new(5, 10), Position::new(6, 10));

            assert_eq!(event.enemy_identifier(), identifier);
            assert_eq!(event.from(), Position::new(5, 10));
            assert_eq!(event.to(), Position::new(6, 10));
        }

        #[rstest]
        fn distance_calculates_manhattan_distance() {
            let event = EnemyMoved::new(
                EntityIdentifier::new(),
                Position::new(0, 0),
                Position::new(3, 4),
            );

            assert_eq!(event.distance().value(), 7);
        }

        #[rstest]
        fn distance_zero_for_same_position() {
            let position = Position::new(5, 5);
            let event = EnemyMoved::new(EntityIdentifier::new(), position, position);

            assert_eq!(event.distance().value(), 0);
        }

        #[rstest]
        fn clone_preserves_values() {
            let event = EnemyMoved::new(
                EntityIdentifier::new(),
                Position::new(0, 0),
                Position::new(1, 1),
            );
            let cloned = event;

            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // EnemyAttacked Tests
    // =========================================================================

    mod enemy_attacked {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let identifier = EntityIdentifier::new();
            let damage = Damage::new(50);
            let event = EnemyAttacked::new(identifier, damage);

            assert_eq!(event.enemy_identifier(), identifier);
            assert_eq!(event.damage(), damage);
        }

        #[rstest]
        fn was_blocked_false_for_damage() {
            let event = EnemyAttacked::new(EntityIdentifier::new(), Damage::new(50));
            assert!(!event.was_blocked());
        }

        #[rstest]
        fn was_blocked_true_for_zero_damage() {
            let event = EnemyAttacked::new(EntityIdentifier::new(), Damage::zero());
            assert!(event.was_blocked());
        }

        #[rstest]
        fn clone_preserves_values() {
            let event = EnemyAttacked::new(EntityIdentifier::new(), Damage::new(100));
            let cloned = event;

            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // EnemyDied Tests
    // =========================================================================

    mod enemy_died {
        use super::*;
        use crate::enemy::LootEntry;
        use crate::item::ItemIdentifier;

        #[rstest]
        fn new_creates_event() {
            let identifier = EntityIdentifier::new();
            let loot = LootTable::empty();
            let event = EnemyDied::new(identifier, loot.clone());

            assert_eq!(event.enemy_identifier(), identifier);
            assert_eq!(*event.loot(), loot);
        }

        #[rstest]
        fn has_loot_false_for_empty() {
            let event = EnemyDied::new(EntityIdentifier::new(), LootTable::empty());
            assert!(!event.has_loot());
        }

        #[rstest]
        fn has_loot_true_with_entries() {
            let item_identifier = ItemIdentifier::new();
            let entry = LootEntry::new(item_identifier, 0.5, 1, 1).unwrap();
            let loot = LootTable::empty().with_entry(entry);
            let event = EnemyDied::new(EntityIdentifier::new(), loot);

            assert!(event.has_loot());
        }

        #[rstest]
        fn clone_preserves_values() {
            let event = EnemyDied::new(EntityIdentifier::new(), LootTable::empty());
            let cloned = event.clone();

            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // EnemyEvent Tests
    // =========================================================================

    mod enemy_event {
        use super::*;

        fn create_spawn_event() -> EnemySpawned {
            EnemySpawned::new(
                EntityIdentifier::new(),
                EnemyType::Goblin,
                Position::new(5, 10),
            )
        }

        fn create_move_event() -> EnemyMoved {
            EnemyMoved::new(
                EntityIdentifier::new(),
                Position::new(0, 0),
                Position::new(1, 1),
            )
        }

        fn create_attack_event() -> EnemyAttacked {
            EnemyAttacked::new(EntityIdentifier::new(), Damage::new(50))
        }

        fn create_death_event() -> EnemyDied {
            EnemyDied::new(EntityIdentifier::new(), LootTable::empty())
        }

        #[rstest]
        fn from_spawned() {
            let spawned = create_spawn_event();
            let identifier = spawned.enemy_identifier();
            let event: EnemyEvent = spawned.into();

            assert!(event.is_spawn());
            assert_eq!(event.enemy_identifier(), identifier);
        }

        #[rstest]
        fn from_moved() {
            let moved = create_move_event();
            let identifier = moved.enemy_identifier();
            let event: EnemyEvent = moved.into();

            assert!(event.is_movement());
            assert_eq!(event.enemy_identifier(), identifier);
        }

        #[rstest]
        fn from_attacked() {
            let attacked = create_attack_event();
            let identifier = attacked.enemy_identifier();
            let event: EnemyEvent = attacked.into();

            assert!(event.is_attack());
            assert_eq!(event.enemy_identifier(), identifier);
        }

        #[rstest]
        fn from_died() {
            let died = create_death_event();
            let identifier = died.enemy_identifier();
            let event: EnemyEvent = died.into();

            assert!(event.is_death());
            assert_eq!(event.enemy_identifier(), identifier);
        }

        #[rstest]
        fn is_spawn_true_for_spawned() {
            let event: EnemyEvent = create_spawn_event().into();
            assert!(event.is_spawn());
            assert!(!event.is_movement());
            assert!(!event.is_attack());
            assert!(!event.is_death());
        }

        #[rstest]
        fn is_movement_true_for_moved() {
            let event: EnemyEvent = create_move_event().into();
            assert!(!event.is_spawn());
            assert!(event.is_movement());
            assert!(!event.is_attack());
            assert!(!event.is_death());
        }

        #[rstest]
        fn is_attack_true_for_attacked() {
            let event: EnemyEvent = create_attack_event().into();
            assert!(!event.is_spawn());
            assert!(!event.is_movement());
            assert!(event.is_attack());
            assert!(!event.is_death());
        }

        #[rstest]
        fn is_death_true_for_died() {
            let event: EnemyEvent = create_death_event().into();
            assert!(!event.is_spawn());
            assert!(!event.is_movement());
            assert!(!event.is_attack());
            assert!(event.is_death());
        }
    }
}
