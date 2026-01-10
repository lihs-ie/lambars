//! Entity identifier for the enemy domain.
//!
//! This module provides a UUID-based identifier for uniquely identifying
//! entities in the game world, such as enemies and dropped items.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// EntityIdentifier
// =============================================================================

/// A unique identifier for entities in the game world.
///
/// EntityIdentifier is a newtype wrapper around UUID that provides
/// type-safe identification of game entities such as enemies and items.
///
/// # Examples
///
/// ```
/// use roguelike_domain::enemy::EntityIdentifier;
///
/// let identifier = EntityIdentifier::new();
/// println!("Entity ID: {}", identifier);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityIdentifier(Uuid);

impl EntityIdentifier {
    /// Creates a new random EntityIdentifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let identifier = EntityIdentifier::new();
    /// assert!(!identifier.to_string().is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an EntityIdentifier from an existing UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::EntityIdentifier;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let identifier = EntityIdentifier::from_uuid(uuid);
    /// ```
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let identifier = EntityIdentifier::new();
    /// let _uuid = identifier.as_uuid();
    /// ```
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Converts the EntityIdentifier to a hyphenated string.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::enemy::EntityIdentifier;
    ///
    /// let identifier = EntityIdentifier::new();
    /// let string = identifier.to_hyphenated_string();
    /// assert_eq!(string.len(), 36); // UUID format: 8-4-4-4-12
    /// ```
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
