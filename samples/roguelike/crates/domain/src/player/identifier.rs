use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::common::ValidationError;

// =============================================================================
// PlayerIdentifier
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerIdentifier(Uuid);

impl PlayerIdentifier {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for PlayerIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PlayerIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for PlayerIdentifier {
    type Err = ValidationError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(string)
            .map(Self)
            .map_err(|_| ValidationError::invalid_format("player_identifier", "valid UUID format"))
    }
}

impl From<Uuid> for PlayerIdentifier {
    fn from(uuid: Uuid) -> Self {
        Self::from_uuid(uuid)
    }
}

// =============================================================================
// PlayerName
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerName(String);

impl PlayerName {
    pub const MIN_LENGTH: usize = 1;
    pub const MAX_LENGTH: usize = 50;

    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::empty_value("player_name"));
        }

        if trimmed.len() > Self::MAX_LENGTH {
            return Err(ValidationError::out_of_range(
                "player_name",
                Self::MIN_LENGTH,
                Self::MAX_LENGTH,
                trimmed.len(),
            ));
        }

        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for PlayerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl AsRef<str> for PlayerName {
    fn as_ref(&self) -> &str {
        &self.0
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
    // PlayerIdentifier Tests
    // =========================================================================

    mod player_identifier {
        use super::*;

        #[rstest]
        fn new_creates_unique_identifier() {
            let identifier1 = PlayerIdentifier::new();
            let identifier2 = PlayerIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn from_uuid_creates_identifier() {
            let uuid = Uuid::new_v4();
            let identifier = PlayerIdentifier::from_uuid(uuid);
            assert_eq!(identifier.value(), uuid);
        }

        #[rstest]
        fn value_returns_inner_uuid() {
            let uuid = Uuid::new_v4();
            let identifier = PlayerIdentifier::from_uuid(uuid);
            assert_eq!(identifier.value(), uuid);
        }

        #[rstest]
        fn default_creates_new_identifier() {
            let identifier = PlayerIdentifier::default();
            // Just verify it doesn't panic and creates a valid identifier
            let _ = identifier.value();
        }

        #[rstest]
        fn display_format() {
            let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
            let identifier = PlayerIdentifier::from_uuid(uuid);
            assert_eq!(
                format!("{}", identifier),
                "550e8400-e29b-41d4-a716-446655440000"
            );
        }

        #[rstest]
        fn from_str_valid_uuid() {
            let result = "550e8400-e29b-41d4-a716-446655440000".parse::<PlayerIdentifier>();
            assert!(result.is_ok());
        }

        #[rstest]
        fn from_str_invalid_uuid() {
            let result = "invalid-uuid".parse::<PlayerIdentifier>();
            assert!(result.is_err());
        }

        #[rstest]
        fn from_str_empty_string() {
            let result = "".parse::<PlayerIdentifier>();
            assert!(result.is_err());
        }

        #[rstest]
        fn from_uuid_trait() {
            let uuid = Uuid::new_v4();
            let identifier: PlayerIdentifier = uuid.into();
            assert_eq!(identifier.value(), uuid);
        }

        #[rstest]
        fn equality() {
            let uuid = Uuid::new_v4();
            let identifier1 = PlayerIdentifier::from_uuid(uuid);
            let identifier2 = PlayerIdentifier::from_uuid(uuid);
            assert_eq!(identifier1, identifier2);
        }

        #[rstest]
        fn inequality() {
            let identifier1 = PlayerIdentifier::new();
            let identifier2 = PlayerIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn clone() {
            let identifier = PlayerIdentifier::new();
            let cloned = identifier;
            assert_eq!(identifier, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let uuid = Uuid::new_v4();
            let identifier1 = PlayerIdentifier::from_uuid(uuid);
            let identifier2 = PlayerIdentifier::from_uuid(uuid);
            let identifier3 = PlayerIdentifier::new();

            let mut set = HashSet::new();
            set.insert(identifier1);

            assert!(set.contains(&identifier2));
            assert!(!set.contains(&identifier3));
        }

        #[rstest]
        fn debug_format() {
            let identifier = PlayerIdentifier::new();
            let debug_string = format!("{:?}", identifier);
            assert!(debug_string.contains("PlayerIdentifier"));
        }
    }

    // =========================================================================
    // PlayerName Tests
    // =========================================================================

    mod player_name {
        use super::*;

        #[rstest]
        fn new_valid_name() {
            let name = PlayerName::new("Hero").unwrap();
            assert_eq!(name.value(), "Hero");
        }

        #[rstest]
        fn new_single_character() {
            let name = PlayerName::new("A").unwrap();
            assert_eq!(name.value(), "A");
        }

        #[rstest]
        fn new_max_length() {
            let long_name = "a".repeat(PlayerName::MAX_LENGTH);
            let name = PlayerName::new(&long_name).unwrap();
            assert_eq!(name.len(), PlayerName::MAX_LENGTH);
        }

        #[rstest]
        fn new_empty_fails() {
            let result = PlayerName::new("");
            assert!(result.is_err());
        }

        #[rstest]
        fn new_whitespace_only_fails() {
            let result = PlayerName::new("   ");
            assert!(result.is_err());
        }

        #[rstest]
        fn new_exceeds_max_length_fails() {
            let long_name = "a".repeat(PlayerName::MAX_LENGTH + 1);
            let result = PlayerName::new(&long_name);
            assert!(result.is_err());
        }

        #[rstest]
        fn new_trims_whitespace() {
            let name = PlayerName::new("  Hero  ").unwrap();
            assert_eq!(name.value(), "Hero");
        }

        #[rstest]
        fn value_returns_name() {
            let name = PlayerName::new("Warrior").unwrap();
            assert_eq!(name.value(), "Warrior");
        }

        #[rstest]
        fn len_returns_length() {
            let name = PlayerName::new("Mage").unwrap();
            assert_eq!(name.len(), 4);
        }

        #[rstest]
        fn is_empty_returns_false() {
            let name = PlayerName::new("Knight").unwrap();
            assert!(!name.is_empty());
        }

        #[rstest]
        fn display_format() {
            let name = PlayerName::new("Ranger").unwrap();
            assert_eq!(format!("{}", name), "Ranger");
        }

        #[rstest]
        fn as_ref_returns_str() {
            let name = PlayerName::new("Cleric").unwrap();
            let name_ref: &str = name.as_ref();
            assert_eq!(name_ref, "Cleric");
        }

        #[rstest]
        fn equality() {
            let name1 = PlayerName::new("Hero").unwrap();
            let name2 = PlayerName::new("Hero").unwrap();
            assert_eq!(name1, name2);
        }

        #[rstest]
        fn inequality() {
            let name1 = PlayerName::new("Hero").unwrap();
            let name2 = PlayerName::new("Villain").unwrap();
            assert_ne!(name1, name2);
        }

        #[rstest]
        fn clone() {
            let name = PlayerName::new("Paladin").unwrap();
            let cloned = name.clone();
            assert_eq!(name, cloned);
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let name1 = PlayerName::new("Druid").unwrap();
            let name2 = PlayerName::new("Druid").unwrap();
            let name3 = PlayerName::new("Bard").unwrap();

            let mut set = HashSet::new();
            set.insert(name1.clone());

            assert!(set.contains(&name2));
            assert!(!set.contains(&name3));
        }

        #[rstest]
        fn debug_format() {
            let name = PlayerName::new("Test").unwrap();
            let debug_string = format!("{:?}", name);
            assert!(debug_string.contains("PlayerName"));
            assert!(debug_string.contains("Test"));
        }

        #[rstest]
        fn new_with_unicode_characters() {
            let name = PlayerName::new("勇者").unwrap();
            assert_eq!(name.value(), "勇者");
        }

        #[rstest]
        fn new_with_special_characters() {
            let name = PlayerName::new("Hero-01_Test").unwrap();
            assert_eq!(name.value(), "Hero-01_Test");
        }

        #[rstest]
        fn new_from_string() {
            let input = String::from("Hero");
            let name = PlayerName::new(input).unwrap();
            assert_eq!(name.value(), "Hero");
        }

        #[rstest]
        fn error_message_for_empty() {
            let result = PlayerName::new("");
            let error = result.unwrap_err();
            assert_eq!(error.field(), "player_name");
        }

        #[rstest]
        fn error_message_for_too_long() {
            let long_name = "a".repeat(PlayerName::MAX_LENGTH + 1);
            let result = PlayerName::new(&long_name);
            let error = result.unwrap_err();
            assert_eq!(error.field(), "player_name");
        }
    }
}
