use crate::common::TurnCount;

use super::Command;

// =============================================================================
// ValidatedCommand
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCommand {
    command: Command,
    validated_at: TurnCount,
}

impl ValidatedCommand {
    #[must_use]
    pub const fn new(command: Command, validated_at: TurnCount) -> Self {
        Self {
            command,
            validated_at,
        }
    }

    #[must_use]
    pub const fn command(&self) -> &Command {
        &self.command
    }

    #[must_use]
    pub const fn validated_at(&self) -> &TurnCount {
        &self.validated_at
    }

    #[must_use]
    pub fn into_command(self) -> Command {
        self.command
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Direction;
    use rstest::rstest;

    mod validated_command {
        use super::*;

        #[rstest]
        fn new_creates_validated_command() {
            let command = Command::Wait;
            let turn = TurnCount::new(10);
            let validated = ValidatedCommand::new(command.clone(), turn);

            assert_eq!(*validated.command(), command);
            assert_eq!(*validated.validated_at(), turn);
        }

        #[rstest]
        fn command_returns_reference() {
            let command = Command::Move(Direction::Up);
            let validated = ValidatedCommand::new(command.clone(), TurnCount::zero());

            assert_eq!(*validated.command(), command);
        }

        #[rstest]
        fn validated_at_returns_reference() {
            let turn = TurnCount::new(100);
            let validated = ValidatedCommand::new(Command::Wait, turn);

            assert_eq!(*validated.validated_at(), turn);
        }

        #[rstest]
        fn into_command_returns_inner_command() {
            let command = Command::Descend;
            let validated = ValidatedCommand::new(command.clone(), TurnCount::zero());

            assert_eq!(validated.into_command(), command);
        }

        #[rstest]
        fn clone_creates_equal_validated_command() {
            let command = Command::Wait;
            let validated = ValidatedCommand::new(command, TurnCount::new(5));
            let cloned = validated.clone();

            assert_eq!(validated, cloned);
        }

        #[rstest]
        fn equality_same_command_and_turn() {
            let command = Command::Wait;
            let turn = TurnCount::new(10);
            let validated1 = ValidatedCommand::new(command.clone(), turn);
            let validated2 = ValidatedCommand::new(command, turn);

            assert_eq!(validated1, validated2);
        }

        #[rstest]
        fn inequality_different_command() {
            let turn = TurnCount::new(10);
            let validated1 = ValidatedCommand::new(Command::Wait, turn);
            let validated2 = ValidatedCommand::new(Command::Descend, turn);

            assert_ne!(validated1, validated2);
        }

        #[rstest]
        fn inequality_different_turn() {
            let command = Command::Wait;
            let validated1 = ValidatedCommand::new(command.clone(), TurnCount::new(10));
            let validated2 = ValidatedCommand::new(command, TurnCount::new(20));

            assert_ne!(validated1, validated2);
        }

        #[rstest]
        fn debug_format() {
            let validated = ValidatedCommand::new(Command::Wait, TurnCount::zero());
            let debug_string = format!("{:?}", validated);

            assert!(debug_string.contains("ValidatedCommand"));
            assert!(debug_string.contains("Wait"));
        }

        #[rstest]
        fn with_move_command() {
            let command = Command::Move(Direction::Left);
            let validated = ValidatedCommand::new(command.clone(), TurnCount::new(1));

            assert_eq!(*validated.command(), command);
        }

        #[rstest]
        fn with_zero_turn() {
            let validated = ValidatedCommand::new(Command::Wait, TurnCount::zero());

            assert_eq!(validated.validated_at().value(), 0);
        }

        #[rstest]
        fn with_large_turn() {
            let large_turn = TurnCount::new(u64::MAX);
            let validated = ValidatedCommand::new(Command::Wait, large_turn);

            assert_eq!(validated.validated_at().value(), u64::MAX);
        }
    }
}
