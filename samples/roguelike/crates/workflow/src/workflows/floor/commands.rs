use roguelike_domain::common::Position;
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;

// =============================================================================
// GenerateFloorCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerateFloorCommand {
    game_identifier: GameIdentifier,
    floor_level: u32,
}

impl GenerateFloorCommand {
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
// DescendFloorCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendFloorCommand {
    game_identifier: GameIdentifier,
}

impl DescendFloorCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier) -> Self {
        Self { game_identifier }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }
}

// =============================================================================
// UpdateVisibilityCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateVisibilityCommand {
    game_identifier: GameIdentifier,
}

impl UpdateVisibilityCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier) -> Self {
        Self { game_identifier }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }
}

// =============================================================================
// TriggerTrapCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerTrapCommand {
    game_identifier: GameIdentifier,
    position: Position,
    target: EntityIdentifier,
}

impl TriggerTrapCommand {
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

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }

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
