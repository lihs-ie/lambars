use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;

// =============================================================================
// ProcessEnemyTurnCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEnemyTurnCommand {
    game_identifier: GameIdentifier,
    entity_identifier: EntityIdentifier,
}

impl ProcessEnemyTurnCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, entity_identifier: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            entity_identifier,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn entity_identifier(&self) -> &EntityIdentifier {
        &self.entity_identifier
    }
}

// =============================================================================
// SpawnEnemiesCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnEnemiesCommand {
    game_identifier: GameIdentifier,
    floor_level: u32,
}

impl SpawnEnemiesCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, floor_level: u32) -> Self {
        Self {
            game_identifier,
            floor_level,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn floor_level(&self) -> u32 {
        self.floor_level
    }
}

// =============================================================================
// ProcessEnemyDeathCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEnemyDeathCommand {
    game_identifier: GameIdentifier,
    entity_identifier: EntityIdentifier,
}

impl ProcessEnemyDeathCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, entity_identifier: EntityIdentifier) -> Self {
        Self {
            game_identifier,
            entity_identifier,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

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
