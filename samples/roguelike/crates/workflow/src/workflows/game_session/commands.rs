use roguelike_domain::game_session::{GameIdentifier, GameOutcome, RandomSeed};

// =============================================================================
// CreateGameCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGameCommand {
    player_name: String,
    seed: Option<RandomSeed>,
}

impl CreateGameCommand {
    #[must_use]
    pub const fn new(player_name: String, seed: Option<RandomSeed>) -> Self {
        Self { player_name, seed }
    }

    #[must_use]
    pub const fn with_seed(player_name: String, seed: RandomSeed) -> Self {
        Self {
            player_name,
            seed: Some(seed),
        }
    }

    #[must_use]
    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    #[must_use]
    pub const fn seed(&self) -> Option<RandomSeed> {
        self.seed
    }
}

// =============================================================================
// ResumeGameCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeGameCommand {
    game_identifier: GameIdentifier,
}

impl ResumeGameCommand {
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
// EndGameCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndGameCommand {
    game_identifier: GameIdentifier,
    outcome: GameOutcome,
}

impl EndGameCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, outcome: GameOutcome) -> Self {
        Self {
            game_identifier,
            outcome,
        }
    }

    #[must_use]
    pub const fn victory(game_identifier: GameIdentifier) -> Self {
        Self::new(game_identifier, GameOutcome::Victory)
    }

    #[must_use]
    pub const fn defeat(game_identifier: GameIdentifier) -> Self {
        Self::new(game_identifier, GameOutcome::Defeat)
    }

    #[must_use]
    pub const fn abandoned(game_identifier: GameIdentifier) -> Self {
        Self::new(game_identifier, GameOutcome::Abandoned)
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn outcome(&self) -> &GameOutcome {
        &self.outcome
    }
}

// =============================================================================
// CreateSnapshotCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSnapshotCommand {
    game_identifier: GameIdentifier,
}

impl CreateSnapshotCommand {
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
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // CreateGameCommand Tests
    // =========================================================================

    mod create_game_command {
        use super::*;

        #[rstest]
        fn new_creates_command_without_seed() {
            let command = CreateGameCommand::new("Hero".to_string(), None);
            assert_eq!(command.player_name(), "Hero");
            assert!(command.seed().is_none());
        }

        #[rstest]
        fn new_creates_command_with_seed() {
            let seed = RandomSeed::new(42);
            let command = CreateGameCommand::new("Hero".to_string(), Some(seed));
            assert_eq!(command.player_name(), "Hero");
            assert_eq!(command.seed(), Some(seed));
        }

        #[rstest]
        fn with_seed_creates_command() {
            let seed = RandomSeed::new(12345);
            let command = CreateGameCommand::with_seed("Player".to_string(), seed);
            assert_eq!(command.player_name(), "Player");
            assert_eq!(command.seed(), Some(seed));
        }

        #[rstest]
        fn equality() {
            let command1 = CreateGameCommand::new("Hero".to_string(), None);
            let command2 = CreateGameCommand::new("Hero".to_string(), None);
            let command3 = CreateGameCommand::new("Villain".to_string(), None);
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = CreateGameCommand::new("Hero".to_string(), Some(RandomSeed::new(42)));
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = CreateGameCommand::new("Hero".to_string(), None);
            let debug = format!("{:?}", command);
            assert!(debug.contains("CreateGameCommand"));
            assert!(debug.contains("Hero"));
        }
    }

    // =========================================================================
    // ResumeGameCommand Tests
    // =========================================================================

    mod resume_game_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let command = ResumeGameCommand::new(identifier);
            assert_eq!(command.game_identifier(), &identifier);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let command1 = ResumeGameCommand::new(identifier);
            let command2 = ResumeGameCommand::new(identifier);
            let command3 = ResumeGameCommand::new(GameIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = ResumeGameCommand::new(GameIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = ResumeGameCommand::new(GameIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("ResumeGameCommand"));
        }
    }

    // =========================================================================
    // EndGameCommand Tests
    // =========================================================================

    mod end_game_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::new(identifier, GameOutcome::Victory);
            assert_eq!(command.game_identifier(), &identifier);
            assert_eq!(command.outcome(), &GameOutcome::Victory);
        }

        #[rstest]
        fn victory_creates_victory_command() {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::victory(identifier);
            assert_eq!(command.outcome(), &GameOutcome::Victory);
        }

        #[rstest]
        fn defeat_creates_defeat_command() {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::defeat(identifier);
            assert_eq!(command.outcome(), &GameOutcome::Defeat);
        }

        #[rstest]
        fn abandoned_creates_abandoned_command() {
            let identifier = GameIdentifier::new();
            let command = EndGameCommand::abandoned(identifier);
            assert_eq!(command.outcome(), &GameOutcome::Abandoned);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let command1 = EndGameCommand::new(identifier, GameOutcome::Victory);
            let command2 = EndGameCommand::new(identifier, GameOutcome::Victory);
            let command3 = EndGameCommand::new(identifier, GameOutcome::Defeat);
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = EndGameCommand::new(GameIdentifier::new(), GameOutcome::Victory);
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = EndGameCommand::new(GameIdentifier::new(), GameOutcome::Victory);
            let debug = format!("{:?}", command);
            assert!(debug.contains("EndGameCommand"));
            assert!(debug.contains("Victory"));
        }
    }

    // =========================================================================
    // CreateSnapshotCommand Tests
    // =========================================================================

    mod create_snapshot_command {
        use super::*;

        #[rstest]
        fn new_creates_command() {
            let identifier = GameIdentifier::new();
            let command = CreateSnapshotCommand::new(identifier);
            assert_eq!(command.game_identifier(), &identifier);
        }

        #[rstest]
        fn equality() {
            let identifier = GameIdentifier::new();
            let command1 = CreateSnapshotCommand::new(identifier);
            let command2 = CreateSnapshotCommand::new(identifier);
            let command3 = CreateSnapshotCommand::new(GameIdentifier::new());
            assert_eq!(command1, command2);
            assert_ne!(command1, command3);
        }

        #[rstest]
        fn clone() {
            let command = CreateSnapshotCommand::new(GameIdentifier::new());
            let cloned = command.clone();
            assert_eq!(command, cloned);
        }

        #[rstest]
        fn debug_format() {
            let command = CreateSnapshotCommand::new(GameIdentifier::new());
            let debug = format!("{:?}", command);
            assert!(debug.contains("CreateSnapshotCommand"));
        }
    }
}
