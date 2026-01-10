//! Item domain module.
//!
//! This module contains item-related domain types including:
//! - ItemIdentifier: Unique identifier for items

use std::fmt;
use uuid::Uuid;

// =============================================================================
// ItemIdentifier
// =============================================================================

/// A unique identifier for items in the game.
///
/// ItemIdentifier wraps a UUID to provide type-safe item identification.
/// This is used for all items including weapons, armor, consumables, and materials.
///
/// # Examples
///
/// ```
/// use roguelike_domain::item::ItemIdentifier;
///
/// let identifier = ItemIdentifier::new();
/// println!("Item: {}", identifier);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemIdentifier(Uuid);

impl ItemIdentifier {
    /// Creates a new ItemIdentifier with a randomly generated UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let identifier = ItemIdentifier::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an ItemIdentifier from an existing UUID.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::item::ItemIdentifier;
    /// use uuid::Uuid;
    ///
    /// let uuid = Uuid::new_v4();
    /// let identifier = ItemIdentifier::from_uuid(uuid);
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
    /// use roguelike_domain::item::ItemIdentifier;
    ///
    /// let identifier = ItemIdentifier::new();
    /// let uuid = identifier.as_uuid();
    /// ```
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ItemIdentifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ItemIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    mod item_identifier {
        use super::*;

        #[rstest]
        fn new_creates_unique_identifiers() {
            let identifier1 = ItemIdentifier::new();
            let identifier2 = ItemIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn from_uuid_creates_identifier() {
            let uuid = Uuid::new_v4();
            let identifier = ItemIdentifier::from_uuid(uuid);
            assert_eq!(*identifier.as_uuid(), uuid);
        }

        #[rstest]
        fn as_uuid_returns_underlying_uuid() {
            let uuid = Uuid::new_v4();
            let identifier = ItemIdentifier::from_uuid(uuid);
            assert_eq!(*identifier.as_uuid(), uuid);
        }

        #[rstest]
        fn default_creates_new_identifier() {
            let identifier1 = ItemIdentifier::default();
            let identifier2 = ItemIdentifier::default();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn clone_creates_equal_identifier() {
            let identifier = ItemIdentifier::new();
            let cloned = identifier;
            assert_eq!(identifier, cloned);
        }

        #[rstest]
        fn equality_for_same_uuid() {
            let uuid = Uuid::new_v4();
            let identifier1 = ItemIdentifier::from_uuid(uuid);
            let identifier2 = ItemIdentifier::from_uuid(uuid);
            assert_eq!(identifier1, identifier2);
        }

        #[rstest]
        fn inequality_for_different_uuid() {
            let identifier1 = ItemIdentifier::new();
            let identifier2 = ItemIdentifier::new();
            assert_ne!(identifier1, identifier2);
        }

        #[rstest]
        fn display_format_matches_uuid() {
            let uuid = Uuid::new_v4();
            let identifier = ItemIdentifier::from_uuid(uuid);
            assert_eq!(format!("{}", identifier), format!("{}", uuid));
        }

        #[rstest]
        fn hash_consistency() {
            use std::collections::HashSet;

            let uuid = Uuid::new_v4();
            let identifier1 = ItemIdentifier::from_uuid(uuid);
            let identifier2 = ItemIdentifier::from_uuid(uuid);

            let mut set = HashSet::new();
            set.insert(identifier1);

            assert!(set.contains(&identifier2));
        }

        #[rstest]
        fn debug_format() {
            let identifier = ItemIdentifier::new();
            let debug_string = format!("{:?}", identifier);
            assert!(debug_string.starts_with("ItemIdentifier("));
        }
    }
}
