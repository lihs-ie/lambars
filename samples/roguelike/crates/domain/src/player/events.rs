//! Player domain events.
//!
//! This module provides domain events for the player aggregate:
//!
//! - **PlayerMoved**: Player moved from one position to another
//! - **PlayerAttacked**: Player attacked an entity
//! - **PlayerDamaged**: Player received damage
//! - **PlayerLeveledUp**: Player gained a level
//! - **PlayerDied**: Player died
//! - **ExperienceGained**: Player gained experience points

use std::fmt;

use crate::common::{Damage, Experience, Level, Position};

// =============================================================================
// EntityIdentifier
// =============================================================================

/// Identifier for any entity in the game (player, enemy, etc.).
///
/// This is a placeholder type for now. In a full implementation,
/// this would be a proper sum type that can identify different entity types.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::EntityIdentifier;
///
/// let entity = EntityIdentifier::new("enemy-001");
/// println!("Entity: {}", entity);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityIdentifier(String);

impl EntityIdentifier {
    /// Creates a new entity identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::player::EntityIdentifier;
    ///
    /// let entity = EntityIdentifier::new("player-001");
    /// assert_eq!(entity.value(), "player-001");
    /// ```
    #[must_use]
    pub fn new(identifier: impl Into<String>) -> Self {
        Self(identifier.into())
    }

    /// Returns the identifier value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<String> for EntityIdentifier {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for EntityIdentifier {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

// =============================================================================
// PlayerMoved
// =============================================================================

/// Event emitted when a player moves from one position to another.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Position;
/// use roguelike_domain::player::PlayerMoved;
///
/// let event = PlayerMoved::new(
///     Position::new(0, 0),
///     Position::new(1, 0),
/// );
/// println!("Player moved: {}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerMoved {
    /// The position the player moved from.
    from: Position,
    /// The position the player moved to.
    to: Position,
}

impl PlayerMoved {
    /// Creates a new `PlayerMoved` event.
    #[must_use]
    pub const fn new(from: Position, to: Position) -> Self {
        Self { from, to }
    }

    /// Returns the starting position.
    #[must_use]
    pub const fn from(&self) -> Position {
        self.from
    }

    /// Returns the ending position.
    #[must_use]
    pub const fn to(&self) -> Position {
        self.to
    }

    /// Returns the distance moved (Manhattan distance).
    #[must_use]
    pub fn distance(&self) -> u32 {
        self.from.distance_to(&self.to).value()
    }
}

impl fmt::Display for PlayerMoved {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Player moved from {} to {}", self.from, self.to)
    }
}

// =============================================================================
// PlayerAttacked
// =============================================================================

/// Event emitted when a player attacks an entity.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Damage;
/// use roguelike_domain::player::{PlayerAttacked, EntityIdentifier};
///
/// let event = PlayerAttacked::new(
///     EntityIdentifier::new("enemy-001"),
///     Damage::new(50),
/// );
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAttacked {
    /// The entity that was attacked.
    target: EntityIdentifier,
    /// The amount of damage dealt.
    damage: Damage,
}

impl PlayerAttacked {
    /// Creates a new `PlayerAttacked` event.
    #[must_use]
    pub const fn new(target: EntityIdentifier, damage: Damage) -> Self {
        Self { target, damage }
    }

    /// Returns the target of the attack.
    #[must_use]
    pub fn target(&self) -> &EntityIdentifier {
        &self.target
    }

    /// Returns the damage dealt.
    #[must_use]
    pub const fn damage(&self) -> Damage {
        self.damage
    }
}

impl fmt::Display for PlayerAttacked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Player attacked {} for {} damage",
            self.target, self.damage
        )
    }
}

// =============================================================================
// PlayerDamaged
// =============================================================================

/// Event emitted when a player receives damage.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Damage;
/// use roguelike_domain::player::{PlayerDamaged, EntityIdentifier};
///
/// let event = PlayerDamaged::new(
///     EntityIdentifier::new("enemy-001"),
///     Damage::new(30),
/// );
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDamaged {
    /// The entity that dealt the damage.
    source: EntityIdentifier,
    /// The amount of damage received.
    damage: Damage,
}

impl PlayerDamaged {
    /// Creates a new `PlayerDamaged` event.
    #[must_use]
    pub const fn new(source: EntityIdentifier, damage: Damage) -> Self {
        Self { source, damage }
    }

    /// Returns the source of the damage.
    #[must_use]
    pub fn source(&self) -> &EntityIdentifier {
        &self.source
    }

    /// Returns the damage received.
    #[must_use]
    pub const fn damage(&self) -> Damage {
        self.damage
    }
}

impl fmt::Display for PlayerDamaged {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Player received {} damage from {}",
            self.damage, self.source
        )
    }
}

// =============================================================================
// PlayerLeveledUp
// =============================================================================

/// Event emitted when a player gains a level.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Level;
/// use roguelike_domain::player::PlayerLeveledUp;
///
/// let event = PlayerLeveledUp::new(Level::new(10).unwrap());
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerLeveledUp {
    /// The new level after leveling up.
    new_level: Level,
}

impl PlayerLeveledUp {
    /// Creates a new `PlayerLeveledUp` event.
    #[must_use]
    pub const fn new(new_level: Level) -> Self {
        Self { new_level }
    }

    /// Returns the new level.
    #[must_use]
    pub const fn new_level(&self) -> Level {
        self.new_level
    }
}

impl fmt::Display for PlayerLeveledUp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Player leveled up to {}", self.new_level)
    }
}

// =============================================================================
// PlayerDied
// =============================================================================

/// Event emitted when a player dies.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::PlayerDied;
///
/// let event = PlayerDied::new();
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerDied;

impl PlayerDied {
    /// Creates a new `PlayerDied` event.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl fmt::Display for PlayerDied {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Player died")
    }
}

// =============================================================================
// ExperienceGained
// =============================================================================

/// Event emitted when a player gains experience points.
///
/// # Examples
///
/// ```
/// use roguelike_domain::common::Experience;
/// use roguelike_domain::player::ExperienceGained;
///
/// let event = ExperienceGained::new(
///     Experience::new(100),
///     Experience::new(1500),
/// );
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceGained {
    /// The amount of experience gained.
    amount: Experience,
    /// The total experience after gaining.
    total: Experience,
}

impl ExperienceGained {
    /// Creates a new `ExperienceGained` event.
    #[must_use]
    pub const fn new(amount: Experience, total: Experience) -> Self {
        Self { amount, total }
    }

    /// Returns the amount of experience gained.
    #[must_use]
    pub const fn amount(&self) -> Experience {
        self.amount
    }

    /// Returns the total experience after gaining.
    #[must_use]
    pub const fn total(&self) -> Experience {
        self.total
    }
}

impl fmt::Display for ExperienceGained {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Gained {} experience (total: {})",
            self.amount, self.total
        )
    }
}

// =============================================================================
// PlayerEvent
// =============================================================================

/// Sum type for all player domain events.
///
/// This enum wraps all specific player events into a single type
/// for easier handling and storage.
///
/// # Examples
///
/// ```
/// use roguelike_domain::player::{PlayerEvent, PlayerDied};
///
/// let event = PlayerEvent::Died(PlayerDied::new());
/// println!("{}", event);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerEvent {
    /// Player moved to a new position.
    Moved(PlayerMoved),
    /// Player attacked an entity.
    Attacked(PlayerAttacked),
    /// Player received damage.
    Damaged(PlayerDamaged),
    /// Player leveled up.
    LeveledUp(PlayerLeveledUp),
    /// Player died.
    Died(PlayerDied),
    /// Player gained experience.
    ExperienceGained(ExperienceGained),
}

impl PlayerEvent {
    /// Returns true if this is a movement event.
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        matches!(self, Self::Moved(_))
    }

    /// Returns true if this is a combat-related event.
    #[must_use]
    pub const fn is_combat(&self) -> bool {
        matches!(self, Self::Attacked(_) | Self::Damaged(_) | Self::Died(_))
    }

    /// Returns true if this is a progression-related event.
    #[must_use]
    pub const fn is_progression(&self) -> bool {
        matches!(self, Self::LeveledUp(_) | Self::ExperienceGained(_))
    }
}

impl fmt::Display for PlayerEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Moved(event) => write!(formatter, "{}", event),
            Self::Attacked(event) => write!(formatter, "{}", event),
            Self::Damaged(event) => write!(formatter, "{}", event),
            Self::LeveledUp(event) => write!(formatter, "{}", event),
            Self::Died(event) => write!(formatter, "{}", event),
            Self::ExperienceGained(event) => write!(formatter, "{}", event),
        }
    }
}

impl From<PlayerMoved> for PlayerEvent {
    fn from(event: PlayerMoved) -> Self {
        Self::Moved(event)
    }
}

impl From<PlayerAttacked> for PlayerEvent {
    fn from(event: PlayerAttacked) -> Self {
        Self::Attacked(event)
    }
}

impl From<PlayerDamaged> for PlayerEvent {
    fn from(event: PlayerDamaged) -> Self {
        Self::Damaged(event)
    }
}

impl From<PlayerLeveledUp> for PlayerEvent {
    fn from(event: PlayerLeveledUp) -> Self {
        Self::LeveledUp(event)
    }
}

impl From<PlayerDied> for PlayerEvent {
    fn from(event: PlayerDied) -> Self {
        Self::Died(event)
    }
}

impl From<ExperienceGained> for PlayerEvent {
    fn from(event: ExperienceGained) -> Self {
        Self::ExperienceGained(event)
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
    // EntityIdentifier Tests
    // =========================================================================

    mod entity_identifier {
        use super::*;

        #[rstest]
        fn new_creates_identifier() {
            let entity = EntityIdentifier::new("entity-001");
            assert_eq!(entity.value(), "entity-001");
        }

        #[rstest]
        fn display_format() {
            let entity = EntityIdentifier::new("enemy-001");
            assert_eq!(format!("{}", entity), "enemy-001");
        }

        #[rstest]
        fn from_string() {
            let entity: EntityIdentifier = String::from("player-001").into();
            assert_eq!(entity.value(), "player-001");
        }

        #[rstest]
        fn from_str() {
            let entity: EntityIdentifier = "boss-001".into();
            assert_eq!(entity.value(), "boss-001");
        }

        #[rstest]
        fn equality() {
            let entity1 = EntityIdentifier::new("id");
            let entity2 = EntityIdentifier::new("id");
            let entity3 = EntityIdentifier::new("other");

            assert_eq!(entity1, entity2);
            assert_ne!(entity1, entity3);
        }

        #[rstest]
        fn clone() {
            let entity = EntityIdentifier::new("test");
            let cloned = entity.clone();
            assert_eq!(entity, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let entity1 = EntityIdentifier::new("id");
            let entity2 = EntityIdentifier::new("id");
            let entity3 = EntityIdentifier::new("other");

            let mut set = HashSet::new();
            set.insert(entity1.clone());

            assert!(set.contains(&entity2));
            assert!(!set.contains(&entity3));
        }
    }

    // =========================================================================
    // PlayerMoved Tests
    // =========================================================================

    mod player_moved {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let from = Position::new(0, 0);
            let to = Position::new(1, 0);
            let event = PlayerMoved::new(from, to);

            assert_eq!(event.from(), from);
            assert_eq!(event.to(), to);
        }

        #[rstest]
        fn distance_calculates_correctly() {
            let from = Position::new(0, 0);
            let to = Position::new(3, 4);
            let event = PlayerMoved::new(from, to);

            assert_eq!(event.distance(), 7);
        }

        #[rstest]
        fn distance_zero_for_same_position() {
            let position = Position::new(5, 5);
            let event = PlayerMoved::new(position, position);

            assert_eq!(event.distance(), 0);
        }

        #[rstest]
        fn display_format() {
            let from = Position::new(0, 0);
            let to = Position::new(1, 0);
            let event = PlayerMoved::new(from, to);

            assert_eq!(format!("{}", event), "Player moved from (0, 0) to (1, 0)");
        }

        #[rstest]
        fn equality() {
            let from = Position::new(0, 0);
            let to = Position::new(1, 0);
            let event1 = PlayerMoved::new(from, to);
            let event2 = PlayerMoved::new(from, to);
            let event3 = PlayerMoved::new(to, from);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = PlayerMoved::new(Position::new(0, 0), Position::new(1, 0));
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // PlayerAttacked Tests
    // =========================================================================

    mod player_attacked {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let target = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(50);
            let event = PlayerAttacked::new(target.clone(), damage);

            assert_eq!(event.target(), &target);
            assert_eq!(event.damage(), damage);
        }

        #[rstest]
        fn display_format() {
            let target = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(50);
            let event = PlayerAttacked::new(target, damage);

            assert_eq!(
                format!("{}", event),
                "Player attacked enemy-001 for 50 damage"
            );
        }

        #[rstest]
        fn equality() {
            let target = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(50);
            let event1 = PlayerAttacked::new(target.clone(), damage);
            let event2 = PlayerAttacked::new(target.clone(), damage);
            let event3 = PlayerAttacked::new(target, Damage::new(100));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = PlayerAttacked::new(EntityIdentifier::new("enemy"), Damage::new(10));
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // PlayerDamaged Tests
    // =========================================================================

    mod player_damaged {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let source = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(30);
            let event = PlayerDamaged::new(source.clone(), damage);

            assert_eq!(event.source(), &source);
            assert_eq!(event.damage(), damage);
        }

        #[rstest]
        fn display_format() {
            let source = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(30);
            let event = PlayerDamaged::new(source, damage);

            assert_eq!(
                format!("{}", event),
                "Player received 30 damage from enemy-001"
            );
        }

        #[rstest]
        fn equality() {
            let source = EntityIdentifier::new("enemy-001");
            let damage = Damage::new(30);
            let event1 = PlayerDamaged::new(source.clone(), damage);
            let event2 = PlayerDamaged::new(source.clone(), damage);
            let event3 = PlayerDamaged::new(source, Damage::new(50));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = PlayerDamaged::new(EntityIdentifier::new("source"), Damage::new(20));
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // PlayerLeveledUp Tests
    // =========================================================================

    mod player_leveled_up {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let level = Level::new(10).unwrap();
            let event = PlayerLeveledUp::new(level);

            assert_eq!(event.new_level(), level);
        }

        #[rstest]
        fn display_format() {
            let level = Level::new(10).unwrap();
            let event = PlayerLeveledUp::new(level);

            assert_eq!(format!("{}", event), "Player leveled up to Lv.10");
        }

        #[rstest]
        fn equality() {
            let level = Level::new(10).unwrap();
            let event1 = PlayerLeveledUp::new(level);
            let event2 = PlayerLeveledUp::new(level);
            let event3 = PlayerLeveledUp::new(Level::new(20).unwrap());

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = PlayerLeveledUp::new(Level::new(5).unwrap());
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // PlayerDied Tests
    // =========================================================================

    mod player_died {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let event = PlayerDied::new();
            assert_eq!(event, PlayerDied);
        }

        #[rstest]
        fn default_creates_event() {
            let event = PlayerDied;
            assert_eq!(event, PlayerDied::new());
        }

        #[rstest]
        fn display_format() {
            let event = PlayerDied::new();
            assert_eq!(format!("{}", event), "Player died");
        }

        #[rstest]
        fn equality() {
            let event1 = PlayerDied::new();
            let event2 = PlayerDied::new();
            assert_eq!(event1, event2);
        }

        #[rstest]
        fn clone() {
            let event = PlayerDied::new();
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // ExperienceGained Tests
    // =========================================================================

    mod experience_gained {
        use super::*;

        #[rstest]
        fn new_creates_event() {
            let amount = Experience::new(100);
            let total = Experience::new(1500);
            let event = ExperienceGained::new(amount, total);

            assert_eq!(event.amount(), amount);
            assert_eq!(event.total(), total);
        }

        #[rstest]
        fn display_format() {
            let amount = Experience::new(100);
            let total = Experience::new(1500);
            let event = ExperienceGained::new(amount, total);

            assert_eq!(format!("{}", event), "Gained 100 experience (total: 1500)");
        }

        #[rstest]
        fn equality() {
            let amount = Experience::new(100);
            let total = Experience::new(1500);
            let event1 = ExperienceGained::new(amount, total);
            let event2 = ExperienceGained::new(amount, total);
            let event3 = ExperienceGained::new(Experience::new(200), total);

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = ExperienceGained::new(Experience::new(50), Experience::new(500));
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }
    }

    // =========================================================================
    // PlayerEvent Tests
    // =========================================================================

    mod player_event {
        use super::*;

        #[rstest]
        fn from_player_moved() {
            let inner = PlayerMoved::new(Position::new(0, 0), Position::new(1, 0));
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::Moved(e) if e == inner));
        }

        #[rstest]
        fn from_player_attacked() {
            let inner = PlayerAttacked::new(EntityIdentifier::new("enemy"), Damage::new(50));
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::Attacked(e) if e == inner));
        }

        #[rstest]
        fn from_player_damaged() {
            let inner = PlayerDamaged::new(EntityIdentifier::new("enemy"), Damage::new(30));
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::Damaged(e) if e == inner));
        }

        #[rstest]
        fn from_player_leveled_up() {
            let inner = PlayerLeveledUp::new(Level::new(10).unwrap());
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::LeveledUp(e) if e == inner));
        }

        #[rstest]
        fn from_player_died() {
            let inner = PlayerDied::new();
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::Died(e) if e == inner));
        }

        #[rstest]
        fn from_experience_gained() {
            let inner = ExperienceGained::new(Experience::new(100), Experience::new(1000));
            let event: PlayerEvent = inner.clone().into();
            assert!(matches!(event, PlayerEvent::ExperienceGained(e) if e == inner));
        }

        #[rstest]
        fn is_movement_for_moved() {
            let event =
                PlayerEvent::Moved(PlayerMoved::new(Position::new(0, 0), Position::new(1, 0)));
            assert!(event.is_movement());
        }

        #[rstest]
        fn is_movement_for_others() {
            let event = PlayerEvent::Died(PlayerDied::new());
            assert!(!event.is_movement());
        }

        #[rstest]
        fn is_combat_for_attacked() {
            let event = PlayerEvent::Attacked(PlayerAttacked::new(
                EntityIdentifier::new("enemy"),
                Damage::new(50),
            ));
            assert!(event.is_combat());
        }

        #[rstest]
        fn is_combat_for_damaged() {
            let event = PlayerEvent::Damaged(PlayerDamaged::new(
                EntityIdentifier::new("enemy"),
                Damage::new(30),
            ));
            assert!(event.is_combat());
        }

        #[rstest]
        fn is_combat_for_died() {
            let event = PlayerEvent::Died(PlayerDied::new());
            assert!(event.is_combat());
        }

        #[rstest]
        fn is_combat_for_others() {
            let event =
                PlayerEvent::Moved(PlayerMoved::new(Position::new(0, 0), Position::new(1, 0)));
            assert!(!event.is_combat());
        }

        #[rstest]
        fn is_progression_for_leveled_up() {
            let event = PlayerEvent::LeveledUp(PlayerLeveledUp::new(Level::new(10).unwrap()));
            assert!(event.is_progression());
        }

        #[rstest]
        fn is_progression_for_experience_gained() {
            let event = PlayerEvent::ExperienceGained(ExperienceGained::new(
                Experience::new(100),
                Experience::new(1000),
            ));
            assert!(event.is_progression());
        }

        #[rstest]
        fn is_progression_for_others() {
            let event = PlayerEvent::Died(PlayerDied::new());
            assert!(!event.is_progression());
        }

        #[rstest]
        fn display_for_moved() {
            let event =
                PlayerEvent::Moved(PlayerMoved::new(Position::new(0, 0), Position::new(1, 0)));
            assert_eq!(format!("{}", event), "Player moved from (0, 0) to (1, 0)");
        }

        #[rstest]
        fn display_for_died() {
            let event = PlayerEvent::Died(PlayerDied::new());
            assert_eq!(format!("{}", event), "Player died");
        }

        #[rstest]
        fn equality() {
            let event1 = PlayerEvent::Died(PlayerDied::new());
            let event2 = PlayerEvent::Died(PlayerDied::new());
            let event3 = PlayerEvent::LeveledUp(PlayerLeveledUp::new(Level::new(10).unwrap()));

            assert_eq!(event1, event2);
            assert_ne!(event1, event3);
        }

        #[rstest]
        fn clone() {
            let event = PlayerEvent::Died(PlayerDied::new());
            let cloned = event.clone();
            assert_eq!(event, cloned);
        }

        #[rstest]
        fn debug_format() {
            let event = PlayerEvent::Died(PlayerDied::new());
            let debug_string = format!("{:?}", event);
            assert!(debug_string.contains("Died"));
        }
    }
}
