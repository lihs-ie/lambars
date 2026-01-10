//! Command types for turn workflows.
//!
//! This module defines the input command types for turn operations.
//! Commands are immutable value objects that represent intent to perform
//! turn-related actions.

use roguelike_domain::common::Direction;
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_domain::item::ItemIdentifier;

// =============================================================================
// PlayerCommand
// =============================================================================

/// Represents player actions that can be performed during a turn.
///
/// This enum defines all possible actions a player can take when it is
/// their turn in the game.
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::PlayerCommand;
/// use roguelike_domain::common::Direction;
/// use roguelike_domain::enemy::EntityIdentifier;
/// use roguelike_domain::item::ItemIdentifier;
///
/// // Movement command
/// let move_command = PlayerCommand::Move(Direction::Up);
///
/// // Attack command
/// let target = EntityIdentifier::new();
/// let attack_command = PlayerCommand::Attack(target);
///
/// // Wait command
/// let wait_command = PlayerCommand::Wait;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCommand {
    /// Move the player in a direction.
    Move(Direction),

    /// Attack a specific enemy.
    Attack(EntityIdentifier),

    /// Use a consumable item from inventory.
    UseItem(ItemIdentifier),

    /// Pick up an item from the current tile.
    PickUpItem(ItemIdentifier),

    /// Equip an item from inventory.
    EquipItem(ItemIdentifier),

    /// Wait and pass the turn (grants rest bonus).
    Wait,
}

impl PlayerCommand {
    /// Returns true if this command is a movement action.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::PlayerCommand;
    /// use roguelike_domain::common::Direction;
    ///
    /// assert!(PlayerCommand::Move(Direction::Up).is_movement());
    /// assert!(!PlayerCommand::Wait.is_movement());
    /// ```
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    /// Returns true if this command is an attack action.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::PlayerCommand;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let target = EntityIdentifier::new();
    /// assert!(PlayerCommand::Attack(target).is_attack());
    /// assert!(!PlayerCommand::Wait.is_attack());
    /// ```
    #[must_use]
    pub const fn is_attack(&self) -> bool {
        matches!(self, Self::Attack(_))
    }

    /// Returns true if this command involves items.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::PlayerCommand;
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let item = ItemIdentifier::new();
    /// assert!(PlayerCommand::UseItem(item).is_item_action());
    /// assert!(PlayerCommand::PickUpItem(item).is_item_action());
    /// assert!(PlayerCommand::EquipItem(item).is_item_action());
    /// assert!(!PlayerCommand::Wait.is_item_action());
    /// ```
    #[must_use]
    pub const fn is_item_action(&self) -> bool {
        matches!(
            self,
            Self::UseItem(_) | Self::PickUpItem(_) | Self::EquipItem(_)
        )
    }

    /// Returns true if this is a wait action.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::PlayerCommand;
    ///
    /// assert!(PlayerCommand::Wait.is_wait());
    /// ```
    #[must_use]
    pub const fn is_wait(&self) -> bool {
        matches!(self, Self::Wait)
    }
}

// =============================================================================
// ProcessTurnCommand
// =============================================================================

/// Command for processing a full turn.
///
/// This command triggers the processing of a complete game turn,
/// including the player's action, all enemy actions, and status
/// effect processing.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `player_command`: The action the player wants to perform
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::{ProcessTurnCommand, PlayerCommand};
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::common::Direction;
///
/// let identifier = GameIdentifier::new();
/// let command = ProcessTurnCommand::new(
///     identifier,
///     PlayerCommand::Move(Direction::Up),
/// );
/// assert!(command.player_command().is_movement());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessTurnCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The player's intended action for this turn.
    player_command: PlayerCommand,
}

impl ProcessTurnCommand {
    /// Creates a new process turn command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `player_command` - The action the player wants to perform.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::{ProcessTurnCommand, PlayerCommand};
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::common::Direction;
    ///
    /// let command = ProcessTurnCommand::new(
    ///     GameIdentifier::new(),
    ///     PlayerCommand::Move(Direction::Down),
    /// );
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, player_command: PlayerCommand) -> Self {
        Self {
            game_identifier,
            player_command,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the player command.
    #[must_use]
    pub const fn player_command(&self) -> PlayerCommand {
        self.player_command
    }
}

// =============================================================================
// WaitTurnCommand
// =============================================================================

/// Command for processing a wait/rest turn.
///
/// This command triggers a simplified turn where the player
/// chooses to wait, potentially recovering resources.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::turn::WaitTurnCommand;
/// use roguelike_domain::game_session::GameIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let command = WaitTurnCommand::new(identifier);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTurnCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
}

impl WaitTurnCommand {
    /// Creates a new wait turn command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::turn::WaitTurnCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    ///
    /// let command = WaitTurnCommand::new(GameIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier) -> Self {
        Self { game_identifier }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
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
    // PlayerCommand Tests
    // =========================================================================

    mod player_command {
        use super::*;

        #[rstest]
        fn move_command_is_movement() {
            let command = PlayerCommand::Move(Direction::Up);
            assert!(command.is_movement());
            assert!(!command.is_attack());
            assert!(!command.is_item_action());
            assert!(!command.is_wait());
        }

        #[rstest]
        #[case(Direction::Up)]
        #[case(Direction::Down)]
        #[case(Direction::Left)]
        #[case(Direction::Right)]
        fn move_with_all_directions(#[case] direction: Direction) {
            let command = PlayerCommand::Move(direction);
            assert!(command.is_movement());
        }

        #[rstest]
        fn attack_command_is_attack() {
            let target = EntityIdentifier::new();
            let command = PlayerCommand::Attack(target);
            assert!(command.is_attack());
            assert!(!command.is_movement());
            assert!(!command.is_item_action());
            assert!(!command.is_wait());
        }

        #[rstest]
        fn use_item_command_is_item_action() {
            let item = ItemIdentifier::new();
            let command = PlayerCommand::UseItem(item);
            assert!(command.is_item_action());
            assert!(!command.is_movement());
            assert!(!command.is_attack());
            assert!(!command.is_wait());
        }

        #[rstest]
        fn pick_up_item_command_is_item_action() {
            let item = ItemIdentifier::new();
            let command = PlayerCommand::PickUpItem(item);
            assert!(command.is_item_action());
        }

        #[rstest]
        fn equip_item_command_is_item_action() {
            let item = ItemIdentifier::new();
            let command = PlayerCommand::EquipItem(item);
            assert!(command.is_item_action());
        }

        #[rstest]
        fn wait_command_is_wait() {
            let command = PlayerCommand::Wait;
            assert!(command.is_wait());
            assert!(!command.is_movement());
            assert!(!command.is_attack());
            assert!(!command.is_item_action());
        }

        #[rstest]
        fn equality_for_same_move() {
            let command1 = PlayerCommand::Move(Direction::Up);
            let command2 = PlayerCommand::Move(Direction::Up);
            assert_eq!(command1, command2);
        }

        #[rstest]
        fn inequality_for_different_moves() {
            let command1 = PlayerCommand::Move(Direction::Up);
            let command2 = PlayerCommand::Move(Direction::Down);
            assert_ne!(command1, command2);
        }

        #[rstest]
        fn equality_for_same_attack() {
            let target = EntityIdentifier::new();
            let command1 = PlayerCommand::Attack(target);
            let command2 = PlayerCommand::Attack(target);
            assert_eq!(command1, command2);
        }

        #[rstest]
        fn inequality_for_different_attacks() {
            let target1 = EntityIdentifier::new();
            let target2 = EntityIdentifier::new();
            let command1 = PlayerCommand::Attack(target1);
            let command2 = PlayerCommand::Attack(target2);
            assert_ne!(command1, command2);
        }

        #[rstest]
        fn inequality_between_different_command_types() {
            let command1 = PlayerCommand::Wait;
            let command2 = PlayerCommand::Move(Direction::Up);
            assert_ne!(command1, command2);
        }

        #[rstest]
        fn clone() {
            let command = PlayerCommand::Move(Direction::Up);
            let cloned = command;
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = PlayerCommand::Move(Direction::Up);
            let debug = format!("{:?}", command);
            assert!(debug.contains("Move"));
            assert!(debug.contains("Up"));
        }
    }

    // =========================================================================
    // ProcessTurnCommand Tests
    // =========================================================================

    mod process_turn_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let player_command = PlayerCommand::Move(Direction::Up);
            let command = ProcessTurnCommand::new(game_identifier, player_command);

            assert_eq!(command.game_identifier(), &game_identifier);
            assert_eq!(command.player_command(), player_command);
        }

        #[rstest]
        fn new_with_wait_command() {
            let game_identifier = GameIdentifier::new();
            let command = ProcessTurnCommand::new(game_identifier, PlayerCommand::Wait);

            assert!(command.player_command().is_wait());
        }

        #[rstest]
        fn new_with_attack_command() {
            let game_identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let command = ProcessTurnCommand::new(game_identifier, PlayerCommand::Attack(target));

            assert!(command.player_command().is_attack());
        }

        #[rstest]
        fn new_with_item_commands() {
            let game_identifier = GameIdentifier::new();
            let item = ItemIdentifier::new();

            let use_command =
                ProcessTurnCommand::new(game_identifier, PlayerCommand::UseItem(item));
            let pick_up_command =
                ProcessTurnCommand::new(game_identifier, PlayerCommand::PickUpItem(item));
            let equip_command =
                ProcessTurnCommand::new(game_identifier, PlayerCommand::EquipItem(item));

            assert!(use_command.player_command().is_item_action());
            assert!(pick_up_command.player_command().is_item_action());
            assert!(equip_command.player_command().is_item_action());
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let player_command = PlayerCommand::Move(Direction::Up);

            let command1 = ProcessTurnCommand::new(game_identifier, player_command);
            let command2 = ProcessTurnCommand::new(game_identifier, player_command);
            let command3 = ProcessTurnCommand::new(game_identifier, PlayerCommand::Wait);

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn inequality_for_different_games() {
            let game1 = GameIdentifier::new();
            let game2 = GameIdentifier::new();
            let player_command = PlayerCommand::Wait;

            let command1 = ProcessTurnCommand::new(game1, player_command);
            let command2 = ProcessTurnCommand::new(game2, player_command);

            assert_ne!(command1, command2);
        }

        #[rstest]
        fn clone() {
            let command =
                ProcessTurnCommand::new(GameIdentifier::new(), PlayerCommand::Move(Direction::Up));
            let cloned = command;
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command =
                ProcessTurnCommand::new(GameIdentifier::new(), PlayerCommand::Move(Direction::Up));
            let debug = format!("{:?}", command);
            assert!(debug.contains("ProcessTurnCommand"));
            assert!(debug.contains("Move"));
        }
    }

    // =========================================================================
    // WaitTurnCommand Tests
    // =========================================================================

    mod wait_turn_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let command = WaitTurnCommand::new(game_identifier);

            assert_eq!(command.game_identifier(), &game_identifier);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let command1 = WaitTurnCommand::new(game_identifier);
            let command2 = WaitTurnCommand::new(game_identifier);
            let command3 = WaitTurnCommand::new(GameIdentifier::new());

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = WaitTurnCommand::new(GameIdentifier::new());
            let cloned = command;
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = WaitTurnCommand::new(GameIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("WaitTurnCommand"));
        }
    }
}
