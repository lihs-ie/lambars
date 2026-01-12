use roguelike_domain::common::Direction;
use roguelike_domain::enemy::EntityIdentifier;
use roguelike_domain::game_session::GameIdentifier;
use roguelike_domain::item::ItemIdentifier;

// =============================================================================
// PlayerCommand
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCommand {
    Move(Direction),

    Attack(EntityIdentifier),

    UseItem(ItemIdentifier),

    PickUpItem(ItemIdentifier),

    EquipItem(ItemIdentifier),

    Wait,
}

impl PlayerCommand {
    #[must_use]
    pub const fn is_movement(&self) -> bool {
        matches!(self, Self::Move(_))
    }

    #[must_use]
    pub const fn is_attack(&self) -> bool {
        matches!(self, Self::Attack(_))
    }

    #[must_use]
    pub const fn is_item_action(&self) -> bool {
        matches!(
            self,
            Self::UseItem(_) | Self::PickUpItem(_) | Self::EquipItem(_)
        )
    }

    #[must_use]
    pub const fn is_wait(&self) -> bool {
        matches!(self, Self::Wait)
    }
}

// =============================================================================
// ProcessTurnCommand
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessTurnCommand {
    game_identifier: GameIdentifier,
    player_command: PlayerCommand,
}

impl ProcessTurnCommand {
    #[must_use]
    pub const fn new(game_identifier: GameIdentifier, player_command: PlayerCommand) -> Self {
        Self {
            game_identifier,
            player_command,
        }
    }

    #[must_use]
    pub const fn game_identifier(&self) -> &GameIdentifier {
        &self.game_identifier
    }

    #[must_use]
    pub const fn player_command(&self) -> PlayerCommand {
        self.player_command
    }
}

// =============================================================================
// WaitTurnCommand
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTurnCommand {
    game_identifier: GameIdentifier,
}

impl WaitTurnCommand {
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
