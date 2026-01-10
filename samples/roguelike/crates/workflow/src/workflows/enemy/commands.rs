//! Command types for enemy workflows.
//!
//! This module defines the input command types for enemy operations.
//! Commands are immutable value objects that represent intent to perform
//! enemy-related actions.

use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;

// =============================================================================
// ProcessEnemyTurnCommand
// =============================================================================

/// Command for processing an enemy's turn.
///
/// This command triggers the AI decision-making and action execution
/// for a specific enemy.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `entity_identifier`: The identifier of the enemy to process
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::ProcessEnemyTurnCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let entity = EntityIdentifier::new();
/// let command = ProcessEnemyTurnCommand::new(identifier, entity);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEnemyTurnCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The enemy entity identifier.
    entity_identifier: EntityIdentifier,
}

impl ProcessEnemyTurnCommand {
    /// Creates a new process enemy turn command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `entity_identifier` - The identifier of the enemy to process.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::enemy::ProcessEnemyTurnCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let command = ProcessEnemyTurnCommand::new(GameIdentifier::new(), EntityIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, entity_identifier: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            entity_identifier,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the entity identifier.
    #[must_use]
    pub const fn entity_identifier(&self) -> &EntityIdentifier {
        &self.entity_identifier
    }
}

// =============================================================================
// SpawnEnemiesCommand
// =============================================================================

/// Command for spawning enemies on a floor.
///
/// This command triggers enemy generation based on floor level configuration.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `floor_level`: The floor level determining spawn configuration
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::SpawnEnemiesCommand;
/// use roguelike_domain::game_session::GameIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let command = SpawnEnemiesCommand::new(identifier, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnEnemiesCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The floor level for spawn configuration.
    floor_level: u32,
}

impl SpawnEnemiesCommand {
    /// Creates a new spawn enemies command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `floor_level` - The floor level determining spawn configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::enemy::SpawnEnemiesCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    ///
    /// let command = SpawnEnemiesCommand::new(GameIdentifier::new(), 3);
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
// ProcessEnemyDeathCommand
// =============================================================================

/// Command for processing an enemy's death.
///
/// This command handles loot generation and enemy removal from the session.
///
/// # Fields
///
/// - `game_identifier`: The game session identifier
/// - `entity_identifier`: The identifier of the dead enemy
///
/// # Examples
///
/// ```
/// use roguelike_workflow::workflows::enemy::ProcessEnemyDeathCommand;
/// use roguelike_domain::game_session::GameIdentifier;
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let identifier = GameIdentifier::new();
/// let entity = EntityIdentifier::new();
/// let command = ProcessEnemyDeathCommand::new(identifier, entity);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEnemyDeathCommand {
    /// The game session identifier.
    game_identifier: GameIdentifier,
    /// The dead enemy entity identifier.
    entity_identifier: EntityIdentifier,
}

impl ProcessEnemyDeathCommand {
    /// Creates a new process enemy death command.
    ///
    /// # Arguments
    ///
    /// * `game_identifier` - The game session identifier.
    /// * `entity_identifier` - The identifier of the dead enemy.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_workflow::workflows::enemy::ProcessEnemyDeathCommand;
    /// use roguelike_domain::game_session::GameIdentifier;
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let command = ProcessEnemyDeathCommand::new(GameIdentifier::new(), EntityIdentifier::new());
    /// ```
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, entity_identifier: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            entity_identifier,
        }
    }

    /// Returns the game identifier.
    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    /// Returns the entity identifier.
    #[must_use]
    pub const fn entity_identifier(&self) -> &EntityIdentifier {
        &self.entity_identifier
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
    // ProcessEnemyTurnCommand Tests
    // =========================================================================

    mod process_enemy_turn_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let entity_identifier = EntityIdentifier::new();
            let command = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);

            assert_eq!(command.game_identifier(), &game_identifier);
            assert_eq!(command.entity_identifier(), &entity_identifier);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let entity_identifier = EntityIdentifier::new();
            let command1 = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);
            let command2 = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);
            let command3 = ProcessEnemyTurnCommand::new(game_identifier, EntityIdentifier::new());

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command =
                ProcessEnemyTurnCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command =
                ProcessEnemyTurnCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("ProcessEnemyTurnCommand"));
        }
    }

    // =========================================================================
    // SpawnEnemiesCommand Tests
    // =========================================================================

    mod spawn_enemies_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let command = SpawnEnemiesCommand::new(game_identifier, 5);

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
            let command = SpawnEnemiesCommand::new(game_identifier, floor_level);
            assert_eq!(command.floor_level(), floor_level);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let command1 = SpawnEnemiesCommand::new(game_identifier, 5);
            let command2 = SpawnEnemiesCommand::new(game_identifier, 5);
            let command3 = SpawnEnemiesCommand::new(game_identifier, 10);

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = SpawnEnemiesCommand::new(GameIdentifier::new(), 3);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = SpawnEnemiesCommand::new(GameIdentifier::new(), 7);
            let debug = format!("{:?}", command);
            assert!(debug.contains("SpawnEnemiesCommand"));
            assert!(debug.contains("7"));
        }
    }

    // =========================================================================
    // ProcessEnemyDeathCommand Tests
    // =========================================================================

    mod process_enemy_death_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let game_identifier = GameIdentifier::new();
            let entity_identifier = EntityIdentifier::new();
            let command = ProcessEnemyDeathCommand::new(game_identifier, entity_identifier);

            assert_eq!(command.game_identifier(), &game_identifier);
            assert_eq!(command.entity_identifier(), &entity_identifier);
        }

        #[rstest]
        fn equality() {
            let game_identifier = GameIdentifier::new();
            let entity_identifier = EntityIdentifier::new();
            let command1 = ProcessEnemyDeathCommand::new(game_identifier, entity_identifier);
            let command2 = ProcessEnemyDeathCommand::new(game_identifier, entity_identifier);
            let command3 = ProcessEnemyDeathCommand::new(game_identifier, EntityIdentifier::new());

            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command =
                ProcessEnemyDeathCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command =
                ProcessEnemyDeathCommand::new(GameIdentifier::new(), EntityIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("ProcessEnemyDeathCommand"));
        }
    }
}
