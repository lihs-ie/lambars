use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::common::ValidationError;

// =============================================================================
// GameIdentifier
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameIdentifier(Uuid);

impl GameIdentifier {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Result<Self, ValidationError> {
        if uuid.is_nil() {
            return Err(ValidationError::empty_value("game_identifier"));
        }
        Ok(Self(uuid))
    }

    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    #[must_use]
    pub fn to_hyphenated_string(&self) -> String {
        self.0.hyphenated().to_string()
    }
}

impl Default for GameIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GameIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0.hyphenated())
    }
}

impl FromStr for GameIdentifier {
    type Err = ValidationError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(string).map_err(|_| {
            ValidationError::invalid_format(
                "game_identifier",
                "valid UUID format (e.g., 550e8400-e29b-41d4-a716-446655440000)",
            )
        })?;

        Self::from_uuid(uuid)
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
    // GameIdentifier Tests
    // =========================================================================

    mod game_identifier {
        use super::*;

        #[rstest]
        fn new_creates_unique_identifier() {
            let identifier1 = GameIdentifier::new();
            let identifier2 = GameIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn new_creates_non_nil_uuid() {
            let identifier = GameIdentifier::new();
            assert!(!identifier.as_uuid().is_nil());
        }

        #[rstest]
        fn from_uuid_valid() {
            let uuid = Uuid::new_v4();
            let identifier = GameIdentifier::from_uuid(uuid).unwrap();
            assert_eq!(identifier.as_uuid(), &uuid);
        }

        #[rstest]
        fn from_uuid_nil_fails() {
            let nil_uuid = Uuid::nil();
            let result = GameIdentifier::from_uuid(nil_uuid);
            assert!(result.is_err());
        }

        #[rstest]
        fn default_creates_unique_identifier() {
            let identifier1 = GameIdentifier::default();
            let identifier2 = GameIdentifier::default();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn display_format() {
            let identifier = GameIdentifier::new();
            let display = format!("{}", identifier);
            // UUID hyphenated format has 36 characters
            assert_eq!(display.len(), 36);
            // Should contain 4 hyphens
            assert_eq!(display.chars().filter(|c| *c == '-').count(), 4);
        }

        #[rstest]
        fn to_hyphenated_string() {
            let identifier = GameIdentifier::new();
            let hyphenated = identifier.to_hyphenated_string();
            assert_eq!(hyphenated.len(), 36);
            assert_eq!(hyphenated.chars().filter(|c| *c == '-').count(), 4);
        }

        #[rstest]
        fn from_str_valid() {
            let uuid_string = "550e8400-e29b-41d4-a716-446655440000";
            let identifier: GameIdentifier = uuid_string.parse().unwrap();
            assert_eq!(identifier.to_string(), uuid_string);
        }

        #[rstest]
        fn from_str_invalid_format() {
            let invalid = "not-a-valid-uuid";
            let result: Result<GameIdentifier, _> = invalid.parse();
            assert!(result.is_err());
        }

        #[rstest]
        fn from_str_nil_uuid_fails() {
            let nil = "00000000-0000-0000-0000-000000000000";
            let result: Result<GameIdentifier, _> = nil.parse();
            assert!(result.is_err());
        }

        #[rstest]
        fn equality() {
            let uuid = Uuid::new_v4();
            let identifier1 = GameIdentifier::from_uuid(uuid).unwrap();
            let identifier2 = GameIdentifier::from_uuid(uuid).unwrap();
            assert_eq!(identifier1, identifier2);
        }

        #[rstest]
        fn inequality() {
            let identifier1 = GameIdentifier::new();
            let identifier2 = GameIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn clone() {
            let identifier = GameIdentifier::new();
            let cloned = identifier;
            assert_eq!(identifier, cloned);
        }

        #[rstest]
        fn copy() {
            let identifier = GameIdentifier::new();
            let copied = identifier;
            // Both should be valid and equal
            assert_eq!(identifier, copied);
        }

        #[rstest]
        fn hash() {
            use std::collections::HashSet;

            let identifier1 = GameIdentifier::new();
            let identifier2 = GameIdentifier::new();
            let identifier1_copy = identifier1;

            let mut set = HashSet::new();
            set.insert(identifier1);
            set.insert(identifier2);
            set.insert(identifier1_copy);

            // Should contain exactly 2 elements (identifier1 and identifier2)
            assert_eq!(set.len(), 2);
        }

        #[rstest]
        fn debug_format() {
            let identifier = GameIdentifier::new();
            let debug_string = format!("{:?}", identifier);
            assert!(debug_string.contains("GameIdentifier"));
        }

        #[rstest]
        fn roundtrip_through_string() {
            let original = GameIdentifier::new();
            let string = original.to_string();
            let parsed: GameIdentifier = string.parse().unwrap();
            assert_eq!(original, parsed);
        }
    }
}
