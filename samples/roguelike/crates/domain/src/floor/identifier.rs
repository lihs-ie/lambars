//! Floor identifier value object.
//!
//! This module provides a unique identifier for dungeon floors.

use std::fmt;

// =============================================================================
// FloorIdentifier
// =============================================================================

/// A unique identifier for a dungeon floor.
///
/// FloorIdentifier is an immutable newtype wrapper around u32 that provides
/// type safety for floor identification. Unlike FloorLevel which represents
/// the depth/progression, FloorIdentifier is used for unique identification.
///
/// # Examples
///
/// ```
/// use roguelike_domain::floor::FloorIdentifier;
///
/// let identifier = FloorIdentifier::new(1);
/// assert_eq!(identifier.value(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FloorIdentifier(u32);

impl FloorIdentifier {
    /// Creates a new FloorIdentifier with the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorIdentifier;
    ///
    /// let identifier = FloorIdentifier::new(42);
    /// assert_eq!(identifier.value(), 42);
    /// ```
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the identifier value.
    ///
    /// # Examples
    ///
    /// ```
    /// use roguelike_domain::floor::FloorIdentifier;
    ///
    /// let identifier = FloorIdentifier::new(10);
    /// assert_eq!(identifier.value(), 10);
    /// ```
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for FloorIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Floor#{}", self.0)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    fn new_creates_identifier() {
        let identifier = FloorIdentifier::new(1);
        assert_eq!(identifier.value(), 1);
    }

    #[rstest]
    fn new_with_zero() {
        let identifier = FloorIdentifier::new(0);
        assert_eq!(identifier.value(), 0);
    }

    #[rstest]
    fn new_with_max_value() {
        let identifier = FloorIdentifier::new(u32::MAX);
        assert_eq!(identifier.value(), u32::MAX);
    }

    #[rstest]
    fn display_format() {
        let identifier = FloorIdentifier::new(42);
        assert_eq!(format!("{}", identifier), "Floor#42");
    }

    #[rstest]
    fn equality() {
        let identifier1 = FloorIdentifier::new(10);
        let identifier2 = FloorIdentifier::new(10);
        let identifier3 = FloorIdentifier::new(20);

        assert_eq!(identifier1, identifier2);
        assert_ne!(identifier1, identifier3);
    }

    #[rstest]
    fn ordering() {
        let small = FloorIdentifier::new(5);
        let large = FloorIdentifier::new(10);

        assert!(small < large);
        assert!(large > small);
    }

    #[rstest]
    fn clone_and_copy() {
        let identifier = FloorIdentifier::new(100);
        let cloned = identifier;
        assert_eq!(identifier, cloned);
    }

    #[rstest]
    fn hash_consistency() {
        use std::collections::HashSet;

        let identifier1 = FloorIdentifier::new(10);
        let identifier2 = FloorIdentifier::new(10);
        let identifier3 = FloorIdentifier::new(20);

        let mut set = HashSet::new();
        set.insert(identifier1);

        assert!(set.contains(&identifier2));
        assert!(!set.contains(&identifier3));
    }

    #[rstest]
    fn debug_format() {
        let identifier = FloorIdentifier::new(5);
        let debug_string = format!("{:?}", identifier);
        assert!(debug_string.contains("FloorIdentifier"));
        assert!(debug_string.contains("5"));
    }
}
