use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// EntityIdentifier
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityIdentifier(Uuid);

impl EntityIdentifier {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    #[must_use]
    pub fn to_hyphenated_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for EntityIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntityIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for EntityIdentifier {
    type Err = uuid::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(string).map(Self)
    }
}

impl From<Uuid> for EntityIdentifier {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EntityIdentifier> for Uuid {
    fn from(identifier: EntityIdentifier) -> Self {
        identifier.0
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
    // Construction Tests
    // =========================================================================

    mod construction {
        use super::*;

        #[rstest]
        fn new_creates_unique_identifiers() {
            let identifier1 = EntityIdentifier::new();
            let identifier2 = EntityIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn from_uuid_preserves_value() {
            let uuid = Uuid::new_v4();
            let identifier = EntityIdentifier::from_uuid(uuid);
            assert_eq!(*identifier.as_uuid(), uuid);
        }

        #[rstest]
        fn default_creates_new_identifier() {
            let identifier1 = EntityIdentifier::default();
            let identifier2 = EntityIdentifier::default();
            assert_ne!(identifier1, identifier2);
        }
    }

    // =========================================================================
    // Conversion Tests
    // =========================================================================

    mod conversion {
        use super::*;

        #[rstest]
        fn to_hyphenated_string_has_correct_length() {
            let identifier = EntityIdentifier::new();
            let string = identifier.to_hyphenated_string();
            assert_eq!(string.len(), 36);
        }

        #[rstest]
        fn to_hyphenated_string_has_correct_format() {
            let identifier = EntityIdentifier::new();
            let string = identifier.to_hyphenated_string();
            let parts: Vec<&str> = string.split('-').collect();
            assert_eq!(parts.len(), 5);
            assert_eq!(parts[0].len(), 8);
            assert_eq!(parts[1].len(), 4);
            assert_eq!(parts[2].len(), 4);
            assert_eq!(parts[3].len(), 4);
            assert_eq!(parts[4].len(), 12);
        }

        #[rstest]
        fn from_str_valid_uuid() {
            let original = EntityIdentifier::new();
            let string = original.to_string();
            let parsed: EntityIdentifier = string.parse().unwrap();
            assert_eq!(original, parsed);
        }

        #[rstest]
        fn from_str_invalid_uuid() {
            let result: Result<EntityIdentifier, _> = "not-a-valid-uuid".parse();
            assert!(result.is_err());
        }

        #[rstest]
        fn from_uuid_trait() {
            let uuid = Uuid::new_v4();
            let identifier: EntityIdentifier = uuid.into();
            assert_eq!(*identifier.as_uuid(), uuid);
        }

        #[rstest]
        fn into_uuid_trait() {
            let identifier = EntityIdentifier::new();
            let uuid_value = *identifier.as_uuid();
            let converted: Uuid = identifier.into();
            assert_eq!(converted, uuid_value);
        }
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    mod display {
        use super::*;

        #[rstest]
        fn display_format_matches_hyphenated() {
            let identifier = EntityIdentifier::new();
            assert_eq!(format!("{}", identifier), identifier.to_hyphenated_string());
        }
    }

    // =========================================================================
    // Equality and Hash Tests
    // =========================================================================

    mod equality_and_hash {
        use super::*;
        use std::collections::HashSet;

        #[rstest]
        fn equality_same_uuid() {
            let uuid = Uuid::new_v4();
            let identifier1 = EntityIdentifier::from_uuid(uuid);
            let identifier2 = EntityIdentifier::from_uuid(uuid);
            assert_eq!(identifier1, identifier2);
        }

        #[rstest]
        fn equality_different_uuid() {
            let identifier1 = EntityIdentifier::new();
            let identifier2 = EntityIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn hash_consistency() {
            let uuid = Uuid::new_v4();
            let identifier1 = EntityIdentifier::from_uuid(uuid);
            let identifier2 = EntityIdentifier::from_uuid(uuid);

            let mut set = HashSet::new();
            set.insert(identifier1);

            assert!(set.contains(&identifier2));
        }

        #[rstest]
        fn hash_uniqueness() {
            let identifier1 = EntityIdentifier::new();
            let identifier2 = EntityIdentifier::new();

            let mut set = HashSet::new();
            set.insert(identifier1);

            assert!(!set.contains(&identifier2));
        }
    }

    // =========================================================================
    // Clone and Copy Tests
    // =========================================================================

    mod clone_and_copy {
        use super::*;

        #[rstest]
        fn copy_preserves_value() {
            let identifier = EntityIdentifier::new();
            let copied: EntityIdentifier = identifier;
            assert_eq!(identifier, copied);
        }
    }
}
