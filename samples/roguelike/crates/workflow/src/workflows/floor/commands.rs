//! Command types for floor workflows.
//!
//! This module defines the input command types for floor operations.
//! Commands are immutable value objects that represent intent to perform
//! floor-related actions.

use roguelike_domain::common::Position;
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;

// =============================================================================
// GenerateFloorCommand
// =============================================================================

/// Command for generating a new floor.
///
/// This command triggers floor generation including rooms, corridors,
/// stairs, items, and traps based on the floor level.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `floor_level`: The level of the floor to generate (1-indexed)
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::GenerateFloorCommand;
/// use roguelike_domain::game_session::GameIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let command = GenerateFloorCommand::new(identifier, 1);
/// assert_eq!(command.floor_level(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateFloorCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The floor level to generate.
    floor_level: u32,
}

impl GenerateFloorCommand {
    /// Creates a new generate floor command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `floor_level` - The floor level to generate.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::floor::GenerateFloorCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    ///
    /// let command = GenerateFloorCommand::new(GameIdentifier::new(), 5);
    /// assert_eq!(command.floor_level(), 5);
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, floor_level: u32) -> Self {
        Self {
            game_identifier,
            floor_level,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the floor level.
    #[must_use]
    pub const fn floor_level(&self) -> u32 {
        self.floor_level
    }
}

// =============================================================================
// DescendFloorCommand
// =============================================================================

/// Command for descending to the next floor.
///
/// This command handles player movement to the next floor, including
/// validation that the player is at a down staircase.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::DescendFloorCommand;
/// use roguelike_domain::game_session::GameIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let command = DescendFloorCommand::new(identifier);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendFloorCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
}

impl DescendFloorCommand {
    /// Creates a new descend floor command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::floor::DescendFloorCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    ///
    /// let command = DescendFloorCommand::new(GameIdentifier::new());
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
// UpdateVisibilityCommand
// =============================================================================

/// Command for updating tile visibility.
///
/// This command recalculates the player's field of view and updates
/// which tiles are visible and explored.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::UpdateVisibilityCommand;
/// use roguelike_domain::game_session::GameIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let command = UpdateVisibilityCommand::new(identifier);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVisibilityCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
}

impl UpdateVisibilityCommand {
    /// Creates a new update visibility command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::floor::UpdateVisibilityCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    ///
    /// let command = UpdateVisibilityCommand::new(GameIdentifier::new());
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
// TriggerTrapCommand
// =============================================================================

/// Command for triggering a trap at a position.
///
/// This command processes trap activation when an entity steps on a trap tile.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `position`: The position of the trap
/// - `target`: The entity that triggered the trap
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::floor::TriggerTrapCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::enemy::EntityIdentifier;
/// use roguelike_domain::common::Position;
///
/// let identifier = GameIdentifier::new();
/// let target = EntityIdentifier::new();
/// let position = Position::new(10, 10);
/// let command = TriggerTrapCommand::new(identifier, position, target);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerTrapCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The position of the trap.
    position: Position,
    /// The entity that triggered the trap.
    target: EntityIdentifier,
}

impl TriggerTrapCommand {
    /// Creates a new trigger trap command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `position` - The position of the trap.
    /// * `target` - The entity that triggered the trap.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::floor::TriggerTrapCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::enemy::EntityIdentifier;
    /// use roguelike_domain::common::Position;
    ///
    /// let command = TriggerTrapCommand::new(
    ///     GameIdentifier::new(),
    ///     Position::new(5, 5),
    ///     EntityIdentifier::new(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(
        game_identifier: GameIdentifier,
        position: Position,
        target: EntityIdentifier,
    ) -> Self {
        Self {
            game_identifier,
            position,
            target,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the trap position.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

    /// Returns the target entity identifier.
    #[must_use]
    pub const fn target(&self) -> EntityIdentifier {
        self.target
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
    // GenerateFloorCommand Tests
    // =========================================================================

    mod generate_floor_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let command = GenerateFloorCommand::new(game_identifier, 5);

            assert_eq!(command.game_identifier(), &game_identifier);
            assert_eq!(command.floor_level(), 5);
        }

        #[rstest]
        #[case(1)]
        #[case(5)]
        #[case(10)]
        #[case(99)]
        fn new_with_various_floor_levels(#[case] floor_level: u32) {
            let game_identifier = GameIdentifier::new();
            let command = GenerateFloorCommand::new(game_identifier, floor_level);
            assert_eq!(command.floor_level(), floor_level);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let command1 = GenerateFloorCommand::new(game_identifier, 5);
            let command2 = GenerateFloorCommand::new(game_identifier, 5);
            let command3 = GenerateFloorCommand::new(game_identifier, 10);

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = GenerateFloorCommand::new(GameIdentifier::new(), 3);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = GenerateFloorCommand::new(GameIdentifier::new(), 7);
            let debug = format!("{:?}", command);
            assert!(debug.contains("GenerateFloorCommand"));
            assert!(debug.contains("7"));
        }
    }

    // =========================================================================
    // DescendFloorCommand Tests
    // =========================================================================

    mod descend_floor_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let command = DescendFloorCommand::new(game_identifier);

            assert_eq!(command.game_identifier(), &game_identifier);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let command1 = DescendFloorCommand::new(game_identifier);
            let command2 = DescendFloorCommand::new(game_identifier);
            let command3 = DescendFloorCommand::new(GameIdentifier::new());

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = DescendFloorCommand::new(GameIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = DescendFloorCommand::new(GameIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("DescendFloorCommand"));
        }
    }

    // =========================================================================
    // UpdateVisibilityCommand Tests
    // =========================================================================

    mod update_visibility_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let command = UpdateVisibilityCommand::new(game_identifier);

            assert_eq!(command.game_identifier(), &game_identifier);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let command1 = UpdateVisibilityCommand::new(game_identifier);
            let command2 = UpdateVisibilityCommand::new(game_identifier);
            let command3 = UpdateVisibilityCommand::new(GameIdentifier::new());

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = UpdateVisibilityCommand::new(GameIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = UpdateVisibilityCommand::new(GameIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("UpdateVisibilityCommand"));
        }
    }

    // =========================================================================
    // TriggerTrapCommand Tests
    // =========================================================================

    mod trigger_trap_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let position = Position::new(10, 20);
            let command = TriggerTrapCommand::new(game_identifier, position, target);

            assert_eq!(command.game_identifier(), &game_identifier);
            assert_eq!(command.position(), position);
            assert_eq!(command.target(), target);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let target = EntityIdentifier::new();
            let position = Position::new(10, 20);
            let command1 = TriggerTrapCommand::new(game_identifier, position, target);
            let command2 = TriggerTrapCommand::new(game_identifier, position, target);
            let command3 = TriggerTrapCommand::new(game_identifier, Position::new(5, 5), target);

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = TriggerTrapCommand::new(
                GameIdentifier::new(),
                Position::new(5, 5),
                EntityIdentifier::new(),
            );
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = TriggerTrapCommand::new(
                GameIdentifier::new(),
                Position::new(10, 10),
                EntityIdentifier::new(),
            );
            let debug = format!("{:?}", command);
            assert!(debug.contains("TriggerTrapCommand"));
            assert!(debug.contains("10"));
        }
    }
}
