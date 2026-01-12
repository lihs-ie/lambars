use std::fmt;

// =============================================================================
// CommandError
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    InvalidCommand { reason: String },

    CommandNotAllowed { command: String, reason: String },

    TargetRequired,

    DirectionRequired,
}

impl CommandError {
    #[must_use]
    pub fn invalid_command(reason: impl Into<String>) -> Self {
        Self::InvalidCommand {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn command_not_allowed(command: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::CommandNotAllowed {
            command: command.into(),
            reason: reason.into(),
        }
    }

    #[must_use]
    pub const fn target_required() -> Self {
        Self::TargetRequired
    }

    #[must_use]
    pub const fn direction_required() -> Self {
        Self::DirectionRequired
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand { reason } => {
                write!(formatter, "Invalid command: {}", reason)
            }
            Self::CommandNotAllowed { command, reason } => {
                write!(formatter, "Command '{}' not allowed: {}", command, reason)
            }
            Self::TargetRequired => {
                write!(formatter, "Target required for this command")
            }
            Self::DirectionRequired => {
                write!(formatter, "Direction required for this command")
            }
        }
    }
}

impl std::error::Error for CommandError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod command_error {
        use super::*;

        #[rstest]
        fn invalid_command_constructor() {
            let error = CommandError::invalid_command("test reason");
            assert_eq!(
                error,
                CommandError::InvalidCommand {
                    reason: "test reason".to_string()
                }
            );
        }

        #[rstest]
        fn invalid_command_with_string() {
            let error = CommandError::invalid_command(String::from("test reason"));
            assert_eq!(
                error,
                CommandError::InvalidCommand {
                    reason: "test reason".to_string()
                }
            );
        }

        #[rstest]
        fn command_not_allowed_constructor() {
            let error = CommandError::command_not_allowed("Move", "Wall blocking");
            assert_eq!(
                error,
                CommandError::CommandNotAllowed {
                    command: "Move".to_string(),
                    reason: "Wall blocking".to_string()
                }
            );
        }

        #[rstest]
        fn command_not_allowed_with_strings() {
            let error = CommandError::command_not_allowed(
                String::from("Attack"),
                String::from("No target in range"),
            );
            assert_eq!(
                error,
                CommandError::CommandNotAllowed {
                    command: "Attack".to_string(),
                    reason: "No target in range".to_string()
                }
            );
        }

        #[rstest]
        fn target_required_constructor() {
            let error = CommandError::target_required();
            assert_eq!(error, CommandError::TargetRequired);
        }

        #[rstest]
        fn direction_required_constructor() {
            let error = CommandError::direction_required();
            assert_eq!(error, CommandError::DirectionRequired);
        }

        #[rstest]
        fn display_invalid_command() {
            let error = CommandError::invalid_command("Unknown command type");
            assert_eq!(
                format!("{}", error),
                "Invalid command: Unknown command type"
            );
        }

        #[rstest]
        fn display_command_not_allowed() {
            let error = CommandError::command_not_allowed("Move", "Wall blocking");
            assert_eq!(
                format!("{}", error),
                "Command 'Move' not allowed: Wall blocking"
            );
        }

        #[rstest]
        fn display_target_required() {
            let error = CommandError::target_required();
            assert_eq!(format!("{}", error), "Target required for this command");
        }

        #[rstest]
        fn display_direction_required() {
            let error = CommandError::direction_required();
            assert_eq!(format!("{}", error), "Direction required for this command");
        }

        #[rstest]
        fn debug_format() {
            let error = CommandError::invalid_command("test");
            let debug_string = format!("{:?}", error);
            assert!(debug_string.contains("InvalidCommand"));
            assert!(debug_string.contains("test"));
        }

        #[rstest]
        fn clone() {
            let error = CommandError::invalid_command("test");
            let cloned = error.clone();
            assert_eq!(error, cloned);
        }

        #[rstest]
        fn equality_same_variant() {
            let error1 = CommandError::invalid_command("test");
            let error2 = CommandError::invalid_command("test");
            assert_eq!(error1, error2);
        }

        #[rstest]
        fn inequality_different_variant() {
            let error1 = CommandError::target_required();
            let error2 = CommandError::direction_required();
            assert_ne!(error1, error2);
        }

        #[rstest]
        fn inequality_same_variant_different_reason() {
            let error1 = CommandError::invalid_command("reason1");
            let error2 = CommandError::invalid_command("reason2");
            assert_ne!(error1, error2);
        }

        #[rstest]
        fn implements_error_trait() {
            let error = CommandError::invalid_command("test");
            let _: &dyn std::error::Error = &error;
        }
    }
}
