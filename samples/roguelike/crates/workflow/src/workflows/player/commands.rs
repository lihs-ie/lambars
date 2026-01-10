//! Command types for player workflows.
//!
//! This module defines the input command types for player operations.
//! Commands are immutable value objects that represent user intent.

use roguelike_domain::common::{Damage, Direction};
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_domain::item::ItemIdentifier;

// =============================================================================
// MovePlayerCommand
// =============================================================================

/// Command for moving the player in a direction.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `direction`: The direction to move
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::MovePlayerCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::common::Direction;
///
/// let identifier = GameIdentifier::new();
/// let command = MovePlayerCommand::new(identifier, Direction::Up);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlayerCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The direction to move.
    direction: Direction,
}

impl MovePlayerCommand {
    /// Creates a new move player command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `direction` - The direction to move.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::MovePlayerCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::common::Direction;
    ///
    /// let command = MovePlayerCommand::new(GameIdentifier::new(), Direction::Up);
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, direction: Direction) -> Self {
        Self {
            game_identifier,
            direction,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }
}

// =============================================================================
// AttackEnemyCommand
// =============================================================================

/// Command for attacking an enemy.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `target`: The identifier of the enemy to attack
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::AttackEnemyCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let target = EntityIdentifier::new();
/// let command = AttackEnemyCommand::new(identifier, target);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackEnemyCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The target enemy identifier.
    target: EntityIdentifier,
}

impl AttackEnemyCommand {
    /// Creates a new attack enemy command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `target` - The identifier of the enemy to attack.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::AttackEnemyCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let command = AttackEnemyCommand::new(GameIdentifier::new(), EntityIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, target: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            target,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the target enemy identifier.
    #[must_use]
    pub const fn target(&self) -> &EntityIdentifier {
        &self.target
    }
}

// =============================================================================
// UseItemCommand
// =============================================================================

/// Command for using an item from inventory.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `item_identifier`: The identifier of the item to use
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::UseItemCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::item::ItemIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let item = ItemIdentifier::new();
/// let command = UseItemCommand::new(identifier, item);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseItemCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The item identifier.
    item_identifier: ItemIdentifier,
}

impl UseItemCommand {
    /// Creates a new use item command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `item_identifier` - The identifier of the item to use.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::UseItemCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let command = UseItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// PickUpItemCommand
// =============================================================================

/// Command for picking up an item from the floor.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `item_identifier`: The identifier of the item to pick up
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::PickUpItemCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::item::ItemIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let item = ItemIdentifier::new();
/// let command = PickUpItemCommand::new(identifier, item);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickUpItemCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The item identifier.
    item_identifier: ItemIdentifier,
}

impl PickUpItemCommand {
    /// Creates a new pick up item command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `item_identifier` - The identifier of the item to pick up.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::PickUpItemCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let command = PickUpItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// EquipItemCommand
// =============================================================================

/// Command for equipping an item from inventory.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `item_identifier`: The identifier of the item to equip
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::EquipItemCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::item::ItemIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let item = ItemIdentifier::new();
/// let command = EquipItemCommand::new(identifier, item);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipItemCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The item identifier.
    item_identifier: ItemIdentifier,
}

impl EquipItemCommand {
    /// Creates a new equip item command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `item_identifier` - The identifier of the item to equip.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::EquipItemCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let command = EquipItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, item_identifier: ItemIdentifier) -> Self {
        Self {
            game_identifier,
            item_identifier,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the item identifier.
    #[must_use]
    pub const fn item_identifier(&self) -> &ItemIdentifier {
        &self.item_identifier
    }
}

// =============================================================================
// TakeDamageCommand
// =============================================================================

/// Command for the player taking damage.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `source`: The identifier of the entity causing the damage
/// - `base_damage`: The base damage amount before reductions
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::player::TakeDamageCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::enemy::EntityIdentifier;
/// use roguelike_domain::common::Damage;
///
/// let identifier = GameIdentifier::new();
/// let source = EntityIdentifier::new();
/// let command = TakeDamageCommand::new(identifier, source, Damage::new(10));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeDamageCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The source entity identifier.
    source: EntityIdentifier,
    /// The base damage amount.
    base_damage: Damage,
}

impl TakeDamageCommand {
    /// Creates a new take damage command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `source` - The identifier of the entity causing the damage.
    /// * `base_damage` - The base damage amount.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::player::TakeDamageCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::enemy::EntityIdentifier;
    /// use roguelike_domain::common::Damage;
    ///
    /// let command = TakeDamageCommand::new(
    ///     GameIdentifier::new(),
    ///     EntityIdentifier::new(),
    ///     Damage::new(10)
    /// );
    /// ```
    #[must_use]
    pub const fn new(
        game_identifier: GameIdentifier,
        source: EntityIdentifier,
        base_damage: Damage,
    ) -> Self {
        Self {
            game_identifier,
            source,
            base_damage,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the source entity identifier.
    #[must_use]
    pub const fn source(&self) -> &EntityIdentifier {
        &self.source
    }

    /// Returns the base damage amount.
    #[must_use]
    pub const fn base_damage(&self) -> Damage {
        self.base_damage
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
    // MovePlayerCommand Tests
    // =========================================================================

    mod move_player_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let command = MovePlayerCommand::new(identifier, Direction::Up);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.direction(), Direction::Up);
        }

        #[rstest]
        #[case(Direction::Up)]
        #[case(Direction::Down)]
        #[case(Direction::Left)]
        #[case(Direction::Right)]
        fn new_with_all_directions(#[case] direction: Direction) {
            let identifier = GameIdentifier::new();
            let command = MovePlayerCommand::new(identifier, direction);
            assert_eq!(command.direction(), direction);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let command1 = MovePlayerCommand::new(identifier, Direction::Up);
            let command2 = MovePlayerCommand::new(identifier, Direction::Up);
            let command3 = MovePlayerCommand::new(identifier, Direction::Down);
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = MovePlayerCommand::new(GameIdentifier::new(), Direction::Up);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = MovePlayerCommand::new(GameIdentifier::new(), Direction::Up);
            let debug = format!("{:?}", command);
            assert!(debug.contains("MovePlayerCommand"));
            assert!(debug.contains("Up"));
        }
    }

    // =========================================================================
    // AttackEnemyCommand Tests
    // =========================================================================

    mod attack_enemy_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let command = AttackEnemyCommand::new(identifier, target);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.target(), &target);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let command1 = AttackEnemyCommand::new(identifier, target);
            let command2 = AttackEnemyCommand::new(identifier, target);
            let command3 = AttackEnemyCommand::new(identifier, EntityIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = AttackEnemyCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = AttackEnemyCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("AttackEnemyCommand"));
        }
    }

    // =========================================================================
    // UseItemCommand Tests
    // =========================================================================

    mod use_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = UseItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = UseItemCommand::new(identifier, item);
            let command2 = UseItemCommand::new(identifier, item);
            let command3 = UseItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = UseItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = UseItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("UseItemCommand"));
        }
    }

    // =========================================================================
    // PickUpItemCommand Tests
    // =========================================================================

    mod pick_up_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = PickUpItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = PickUpItemCommand::new(identifier, item);
            let command2 = PickUpItemCommand::new(identifier, item);
            let command3 = PickUpItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = PickUpItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = PickUpItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("PickUpItemCommand"));
        }
    }

    // =========================================================================
    // EquipItemCommand Tests
    // =========================================================================

    mod equip_item_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command = EquipItemCommand::new(identifier, item);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.item_identifier(), &item);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();
            let command1 = EquipItemCommand::new(identifier, item);
            let command2 = EquipItemCommand::new(identifier, item);
            let command3 = EquipItemCommand::new(identifier, ItemIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = EquipItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = EquipItemCommand::new(GameIdentifier::new(), ItemIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("EquipItemCommand"));
        }
    }

    // =========================================================================
    // TakeDamageCommand Tests
    // =========================================================================

    mod take_damage_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let source = EntityIdentifier::new();
            let damage = Damage::new(10);
            let command = TakeDamageCommand::new(identifier, source, damage);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.source(), &source);
            assert_eq!(command.base_damage(), damage);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let source = EntityIdentifier::new();
            let damage = Damage::new(10);
            let command1 = TakeDamageCommand::new(identifier, source, damage);
            let command2 = TakeDamageCommand::new(identifier, source, damage);
            let command3 = TakeDamageCommand::new(identifier, source, Damage::new(20));
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = TakeDamageCommand::new(
                GameIdentifier::new(),
                EntityIdentifier::new(),
                Damage::new(10),
            );
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = TakeDamageCommand::new(
                GameIdentifier::new(),
                EntityIdentifier::new(),
                Damage::new(10),
            );
            let debug = format!("{:?}", command);
            assert!(debug.contains("TakeDamageCommand"));
        }
    }
}
