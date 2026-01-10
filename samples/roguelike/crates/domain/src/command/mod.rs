//! Command domain module.
//!
//! This module contains command-related domain types for the roguelike game:
//!
//! - **Command**: All possible game commands (move, attack, use item, etc.)
//! - **ValidatedCommand**: A command that has passed validation
//! - **CommandError**: Errors that can occur during command processing
//!
//! # Command Processing Pipeline
//!
//! Commands follow a pure functional processing pipeline:
//!
//! 1. Raw input is parsed into a `Command` enum
//! 2. `CommandValidator` validates the command against game state, producing `Either<CommandError, ValidatedCommand>`
//! 3. `CommandExecutor` executes validated commands using `State` monad
//!
//! # Examples
//!
//! ```
//! use roguelike_domain::command::{Command, ValidatedCommand, CommandError};
//! use roguelike_domain::common::{Direction, TurnCount};
//!
//! // Create a move command
//! let command = Command::Move(Direction::Up);
//!
//! // Create a validated command (normally done by CommandValidator)
//! let validated = ValidatedCommand::new(command, TurnCount::new(1));
//!
//! // Access the inner command
//! assert!(matches!(validated.command(), Command::Move(_)));
//! ```

mod errors;
mod validated;

use std::fmt;

use crate::common::Direction;
use crate::enemy::EntityIdentifier;
use crate::item::ItemIdentifier;
use crate::player::EquipmentSlot;

pub use errors::CommandError;
pub use validated::ValidatedCommand;

// =============================================================================
// Command
// =============================================================================

/// All possible game commands that a player can execute.
///
/// Commands represent player intentions in the game. Each command
/// variant may carry data needed for execution (e.g., direction for Move,
/// target for Attack).
///
/// # Variants
///
/// - `Move(Direction)`: Move in a direction
/// - `Attack(EntityIdentifier)`: Attack a specific entity
/// - `UseItem(ItemIdentifier)`: Use an item from inventory
/// - `PickUp(ItemIdentifier)`: Pick up an item from the ground
/// - `Drop(ItemIdentifier)`: Drop an item from inventory
/// - `Equip(ItemIdentifier)`: Equip an item from inventory
/// - `Unequip(EquipmentSlot)`: Unequip from a specific slot
/// - `Wait`: Skip the current turn
/// - `Descend`: Go down to the next floor
/// - `Ascend`: Go up to the previous floor
///
/// # Examples
///
/// ```
/// use roguelike_domain::command::Command;
/// use roguelike_domain::common::Direction;
///
/// let move_command = Command::Move(Direction::Up);
/// let wait_command = Command::Wait;
///
/// // Commands can be pattern matched
/// match move_command {
///     Command::Move(direction) => println!("Moving {:?}", direction),
///     _ => println!("Other command"),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Move the player in a direction.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::common::Direction;
    ///
    /// let command = Command::Move(Direction::Up);
    /// ```
    Move(Direction),

    /// Attack a specific entity.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let target = EntityIdentifier::new();
    /// let command = Command::Attack(target);
    /// ```
    Attack(EntityIdentifier),

    /// Use an item from the player's inventory.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// let command = Command::UseItem(item);
    /// ```
    UseItem(ItemIdentifier),

    /// Pick up an item from the ground.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// let command = Command::PickUp(item);
    /// ```
    PickUp(ItemIdentifier),

    /// Drop an item from the player's inventory.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// let command = Command::Drop(item);
    /// ```
    Drop(ItemIdentifier),

    /// Equip an item from the player's inventory.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// let command = Command::Equip(item);
    /// ```
    Equip(ItemIdentifier),

    /// Unequip an item from a specific equipment slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::player::EquipmentSlot;
    ///
    /// let command = Command::Unequip(EquipmentSlot::Weapon);
    /// ```
    Unequip(EquipmentSlot),

    /// Wait and skip the current turn.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    ///
    /// let command = Command::Wait;
    /// ```
    Wait,

    /// Descend to the next floor.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    ///
    /// let command = Command::Descend;
    /// ```
    Descend,

    /// Ascend to the previous floor.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    ///
    /// let command = Command::Ascend;
    /// ```
    Ascend,
}

impl Command {
    /// Returns true if this command is a movement command.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::common::Direction;
    ///
    /// assert!(Command::Move(Direction::Up).is_movement());
    /// assert!(!Command::Wait.is_movement());
    /// ```
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    /// Returns true if this command is a combat command.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let target = EntityIdentifier::new();
    /// assert!(Command::Attack(target).is_combat());
    /// assert!(!Command::Wait.is_combat());
    /// ```
    #[must_use]
    pub const fn is_combat(&self) -> bool {
        matches!(self, Self::Attack(_))
    }

    /// Returns true if this command is an item-related command.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// assert!(Command::UseItem(item).is_item_command());
    /// assert!(!Command::Wait.is_item_command());
    /// ```
    #[must_use]
    pub const fn is_item_command(&self) -> bool {
        matches!(
            self,
            Self::UseItem(_) | Self::PickUp(_) | Self::Drop(_) | Self::Equip(_) | Self::Unequip(_)
        )
    }

    /// Returns true if this command is a floor transition command.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    ///
    /// assert!(Command::Descend.is_floor_transition());
    /// assert!(Command::Ascend.is_floor_transition());
    /// assert!(!Command::Wait.is_floor_transition());
    /// ```
    #[must_use]
    pub const fn is_floor_transition(&self) -> bool {
        matches!(self, Self::Descend | Self::Ascend)
    }

    /// Returns the name of the command as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::command::Command;
    ///
    /// assert_eq!(Command::Wait.name(), "Wait");
    /// assert_eq!(Command::Descend.name(), "Descend");
    /// ```
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Move(_) => "Move",
            Self::Attack(_) => "Attack",
            Self::UseItem(_) => "UseItem",
            Self::PickUp(_) => "PickUp",
            Self::Drop(_) => "Drop",
            Self::Equip(_) => "Equip",
            Self::Unequip(_) => "Unequip",
            Self::Wait => "Wait",
            Self::Descend => "Descend",
            Self::Ascend => "Ascend",
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move(direction) => write!(formatter, "Move({})", direction),
            Self::Attack(target) => write!(formatter, "Attack({})", target),
            Self::UseItem(item) => write!(formatter, "UseItem({})", item),
            Self::PickUp(item) => write!(formatter, "PickUp({})", item),
            Self::Drop(item) => write!(formatter, "Drop({})", item),
            Self::Equip(item) => write!(formatter, "Equip({})", item),
            Self::Unequip(slot) => write!(formatter, "Unequip({})", slot),
            Self::Wait => write!(formatter, "Wait"),
            Self::Descend => write!(formatter, "Descend"),
            Self::Ascend => write!(formatter, "Ascend"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use uuid::Uuid;

    mod command {
        use super::*;

        // =====================================================================
        // Constructor Tests
        // =====================================================================

        #[rstest]
        fn move_command_with_all_directions() {
            for direction in Direction::all() {
                let command = Command::Move(direction);
                assert!(matches!(command, Command::Move(_)));
            }
        }

        #[rstest]
        fn attack_command_with_entity() {
            let target = EntityIdentifier::new();
            let command = Command::Attack(target);
            assert!(matches!(command, Command::Attack(_)));
        }

        #[rstest]
        fn use_item_command() {
            let item = ItemIdentifier::new();
            let command = Command::UseItem(item);
            assert!(matches!(command, Command::UseItem(_)));
        }

        #[rstest]
        fn pick_up_command() {
            let item = ItemIdentifier::new();
            let command = Command::PickUp(item);
            assert!(matches!(command, Command::PickUp(_)));
        }

        #[rstest]
        fn drop_command() {
            let item = ItemIdentifier::new();
            let command = Command::Drop(item);
            assert!(matches!(command, Command::Drop(_)));
        }

        #[rstest]
        fn equip_command() {
            let item = ItemIdentifier::new();
            let command = Command::Equip(item);
            assert!(matches!(command, Command::Equip(_)));
        }

        #[rstest]
        fn unequip_command_with_all_slots() {
            for slot in EquipmentSlot::all() {
                let command = Command::Unequip(slot);
                assert!(matches!(command, Command::Unequip(_)));
            }
        }

        #[rstest]
        fn wait_command() {
            let command = Command::Wait;
            assert!(matches!(command, Command::Wait));
        }

        #[rstest]
        fn descend_command() {
            let command = Command::Descend;
            assert!(matches!(command, Command::Descend));
        }

        #[rstest]
        fn ascend_command() {
            let command = Command::Ascend;
            assert!(matches!(command, Command::Ascend));
        }

        // =====================================================================
        // is_movement Tests
        // =====================================================================

        #[rstest]
        fn is_movement_for_move_command() {
            let command = Command::Move(Direction::Up);
            assert!(command.is_movement());
        }

        #[rstest]
        fn is_movement_for_other_commands() {
            assert!(!Command::Wait.is_movement());
            assert!(!Command::Descend.is_movement());
            assert!(!Command::Attack(EntityIdentifier::new()).is_movement());
        }

        // =====================================================================
        // is_combat Tests
        // =====================================================================

        #[rstest]
        fn is_combat_for_attack_command() {
            let target = EntityIdentifier::new();
            let command = Command::Attack(target);
            assert!(command.is_combat());
        }

        #[rstest]
        fn is_combat_for_other_commands() {
            assert!(!Command::Wait.is_combat());
            assert!(!Command::Move(Direction::Up).is_combat());
        }

        // =====================================================================
        // is_item_command Tests
        // =====================================================================

        #[rstest]
        fn is_item_command_for_use_item() {
            let item = ItemIdentifier::new();
            assert!(Command::UseItem(item).is_item_command());
        }

        #[rstest]
        fn is_item_command_for_pick_up() {
            let item = ItemIdentifier::new();
            assert!(Command::PickUp(item).is_item_command());
        }

        #[rstest]
        fn is_item_command_for_drop() {
            let item = ItemIdentifier::new();
            assert!(Command::Drop(item).is_item_command());
        }

        #[rstest]
        fn is_item_command_for_equip() {
            let item = ItemIdentifier::new();
            assert!(Command::Equip(item).is_item_command());
        }

        #[rstest]
        fn is_item_command_for_unequip() {
            assert!(Command::Unequip(EquipmentSlot::Weapon).is_item_command());
        }

        #[rstest]
        fn is_item_command_for_other_commands() {
            assert!(!Command::Wait.is_item_command());
            assert!(!Command::Move(Direction::Up).is_item_command());
            assert!(!Command::Attack(EntityIdentifier::new()).is_item_command());
        }

        // =====================================================================
        // is_floor_transition Tests
        // =====================================================================

        #[rstest]
        fn is_floor_transition_for_descend() {
            assert!(Command::Descend.is_floor_transition());
        }

        #[rstest]
        fn is_floor_transition_for_ascend() {
            assert!(Command::Ascend.is_floor_transition());
        }

        #[rstest]
        fn is_floor_transition_for_other_commands() {
            assert!(!Command::Wait.is_floor_transition());
            assert!(!Command::Move(Direction::Up).is_floor_transition());
        }

        // =====================================================================
        // name Tests
        // =====================================================================

        #[rstest]
        #[case(Command::Move(Direction::Up), "Move")]
        #[case(Command::Wait, "Wait")]
        #[case(Command::Descend, "Descend")]
        #[case(Command::Ascend, "Ascend")]
        fn name_returns_correct_string(#[case] command: Command, #[case] expected: &str) {
            assert_eq!(command.name(), expected);
        }

        #[rstest]
        fn name_for_attack() {
            let command = Command::Attack(EntityIdentifier::new());
            assert_eq!(command.name(), "Attack");
        }

        #[rstest]
        fn name_for_item_commands() {
            let item = ItemIdentifier::new();
            assert_eq!(Command::UseItem(item).name(), "UseItem");
            assert_eq!(Command::PickUp(item).name(), "PickUp");
            assert_eq!(Command::Drop(item).name(), "Drop");
            assert_eq!(Command::Equip(item).name(), "Equip");
            assert_eq!(Command::Unequip(EquipmentSlot::Weapon).name(), "Unequip");
        }

        // =====================================================================
        // Display Tests
        // =====================================================================

        #[rstest]
        fn display_move() {
            let command = Command::Move(Direction::Up);
            assert_eq!(format!("{}", command), "Move(Up)");
        }

        #[rstest]
        fn display_attack() {
            let uuid = Uuid::new_v4();
            let target = EntityIdentifier::from_uuid(uuid);
            let command = Command::Attack(target);
            assert_eq!(format!("{}", command), format!("Attack({})", uuid));
        }

        #[rstest]
        fn display_use_item() {
            let uuid = Uuid::new_v4();
            let item = ItemIdentifier::from_uuid(uuid);
            let command = Command::UseItem(item);
            assert_eq!(format!("{}", command), format!("UseItem({})", uuid));
        }

        #[rstest]
        fn display_pick_up() {
            let uuid = Uuid::new_v4();
            let item = ItemIdentifier::from_uuid(uuid);
            let command = Command::PickUp(item);
            assert_eq!(format!("{}", command), format!("PickUp({})", uuid));
        }

        #[rstest]
        fn display_drop() {
            let uuid = Uuid::new_v4();
            let item = ItemIdentifier::from_uuid(uuid);
            let command = Command::Drop(item);
            assert_eq!(format!("{}", command), format!("Drop({})", uuid));
        }

        #[rstest]
        fn display_equip() {
            let uuid = Uuid::new_v4();
            let item = ItemIdentifier::from_uuid(uuid);
            let command = Command::Equip(item);
            assert_eq!(format!("{}", command), format!("Equip({})", uuid));
        }

        #[rstest]
        fn display_unequip() {
            let command = Command::Unequip(EquipmentSlot::Weapon);
            assert_eq!(format!("{}", command), "Unequip(Weapon)");
        }

        #[rstest]
        fn display_wait() {
            assert_eq!(format!("{}", Command::Wait), "Wait");
        }

        #[rstest]
        fn display_descend() {
            assert_eq!(format!("{}", Command::Descend), "Descend");
        }

        #[rstest]
        fn display_ascend() {
            assert_eq!(format!("{}", Command::Ascend), "Ascend");
        }

        // =====================================================================
        // Clone Tests
        // =====================================================================

        #[rstest]
        fn clone_move_command() {
            let command = Command::Move(Direction::Up);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn clone_attack_command() {
            let target = EntityIdentifier::new();
            let command = Command::Attack(target);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn clone_item_command() {
            let item = ItemIdentifier::new();
            let command = Command::UseItem(item);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        // =====================================================================
        // Equality Tests
        // =====================================================================

        #[rstest]
        fn equality_same_move_direction() {
            let command1 = Command::Move(Direction::Up);
            let command2 = Command::Move(Direction::Up);
            assert_eq!(command1, command2);
        }

        #[rstest]
        fn inequality_different_move_direction() {
            let command1 = Command::Move(Direction::Up);
            let command2 = Command::Move(Direction::Down);
            assert_ne!(command1, command2);
        }

        #[rstest]
        fn equality_same_entity() {
            let uuid = Uuid::new_v4();
            let target1 = EntityIdentifier::from_uuid(uuid);
            let target2 = EntityIdentifier::from_uuid(uuid);
            let command1 = Command::Attack(target1);
            let command2 = Command::Attack(target2);
            assert_eq!(command1, command2);
        }

        #[rstest]
        fn inequality_different_entity() {
            let command1 = Command::Attack(EntityIdentifier::new());
            let command2 = Command::Attack(EntityIdentifier::new());
            assert_ne!(command1, command2);
        }

        #[rstest]
        fn equality_unit_variants() {
            assert_eq!(Command::Wait, Command::Wait);
            assert_eq!(Command::Descend, Command::Descend);
            assert_eq!(Command::Ascend, Command::Ascend);
        }

        #[rstest]
        fn inequality_different_variants() {
            assert_ne!(Command::Wait, Command::Descend);
            assert_ne!(Command::Move(Direction::Up), Command::Wait);
        }

        // =====================================================================
        // Debug Tests
        // =====================================================================

        #[rstest]
        fn debug_format() {
            let command = Command::Move(Direction::Up);
            let debug_string = format!("{:?}", command);
            assert!(debug_string.contains("Move"));
            assert!(debug_string.contains("Up"));
        }

        #[rstest]
        fn debug_format_wait() {
            let debug_string = format!("{:?}", Command::Wait);
            assert_eq!(debug_string, "Wait");
        }
    }
}
