//! Error types for the item domain.
//!
//! This module provides error types specific to item operations.

use std::error::Error;
use std::fmt;

// =============================================================================
// ItemError
// =============================================================================

/// Error types for item domain operations.
///
/// These errors represent failures that can occur during item-related operations
/// such as picking up, dropping, using, or equipping items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemError {
    /// The specified item was not found.
    ItemNotFound {
        /// The identifier of the item that was not found.
        item_identifier: String,
    },
    /// The item cannot be used (e.g., trying to use a weapon).
    ItemNotUsable {
        /// The identifier of the item that cannot be used.
        item_identifier: String,
    },
    /// The item cannot be equipped (e.g., trying to equip a consumable).
    ItemNotEquippable {
        /// The identifier of the item that cannot be equipped.
        item_identifier: String,
    },
    /// Adding items would exceed the maximum stack size.
    StackOverflow {
        /// The maximum allowed stack size.
        max_stack: u32,
    },
    /// The item quantity is invalid (e.g., zero or negative).
    InvalidItemQuantity,
}

impl ItemError {
    /// Creates a new `ItemNotFound` error.
    pub fn item_not_found(item_identifier: impl Into<String>) -> Self {
        Self::ItemNotFound {
            item_identifier: item_identifier.into(),
        }
    }

    /// Creates a new `ItemNotUsable` error.
    pub fn item_not_usable(item_identifier: impl Into<String>) -> Self {
        Self::ItemNotUsable {
            item_identifier: item_identifier.into(),
        }
    }

    /// Creates a new `ItemNotEquippable` error.
    pub fn item_not_equippable(item_identifier: impl Into<String>) -> Self {
        Self::ItemNotEquippable {
            item_identifier: item_identifier.into(),
        }
    }

    /// Creates a new `StackOverflow` error.
    #[must_use]
    pub const fn stack_overflow(max_stack: u32) -> Self {
        Self::StackOverflow { max_stack }
    }

    /// Creates a new `InvalidItemQuantity` error.
    #[must_use]
    pub const fn invalid_item_quantity() -> Self {
        Self::InvalidItemQuantity
    }

    /// Returns a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            Self::ItemNotFound { item_identifier } => {
                format!("Item not found: {}", item_identifier)
            }
            Self::ItemNotUsable { item_identifier } => {
                format!("Item cannot be used: {}", item_identifier)
            }
            Self::ItemNotEquippable { item_identifier } => {
                format!("Item cannot be equipped: {}", item_identifier)
            }
            Self::StackOverflow { max_stack } => {
                format!("Stack overflow: maximum stack size is {}", max_stack)
            }
            Self::InvalidItemQuantity => "Invalid item quantity".to_string(),
        }
    }

    /// Returns true if this error indicates an item was not found.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::ItemNotFound { .. })
    }

    /// Returns true if this error is related to item usage.
    #[must_use]
    pub const fn is_usage_error(&self) -> bool {
        matches!(self, Self::ItemNotUsable { .. } | Self::ItemNotEquippable { .. })
    }

    /// Returns true if this error is related to item quantities.
    #[must_use]
    pub const fn is_quantity_error(&self) -> bool {
        matches!(self, Self::StackOverflow { .. } | Self::InvalidItemQuantity)
    }
}

impl fmt::Display for ItemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message())
    }
}

impl Error for ItemError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // Constructor Tests
    // =========================================================================

    #[rstest]
    fn item_not_found_from_string() {
        let error = ItemError::item_not_found("item-123");
        assert!(matches!(error, ItemError::ItemNotFound { item_identifier } if item_identifier == "item-123"));
    }

    #[rstest]
    fn item_not_usable_from_string() {
        let error = ItemError::item_not_usable("weapon-456");
        assert!(matches!(error, ItemError::ItemNotUsable { item_identifier } if item_identifier == "weapon-456"));
    }

    #[rstest]
    fn item_not_equippable_from_string() {
        let error = ItemError::item_not_equippable("potion-789");
        assert!(matches!(error, ItemError::ItemNotEquippable { item_identifier } if item_identifier == "potion-789"));
    }

    #[rstest]
    fn stack_overflow_from_value() {
        let error = ItemError::stack_overflow(99);
        assert!(matches!(error, ItemError::StackOverflow { max_stack } if max_stack == 99));
    }

    #[rstest]
    fn invalid_item_quantity_const() {
        let error = ItemError::invalid_item_quantity();
        assert!(matches!(error, ItemError::InvalidItemQuantity));
    }

    // =========================================================================
    // message Tests
    // =========================================================================

    #[rstest]
    fn message_item_not_found() {
        let error = ItemError::item_not_found("abc-123");
        assert_eq!(error.message(), "Item not found: abc-123");
    }

    #[rstest]
    fn message_item_not_usable() {
        let error = ItemError::item_not_usable("sword-001");
        assert_eq!(error.message(), "Item cannot be used: sword-001");
    }

    #[rstest]
    fn message_item_not_equippable() {
        let error = ItemError::item_not_equippable("potion-001");
        assert_eq!(error.message(), "Item cannot be equipped: potion-001");
    }

    #[rstest]
    fn message_stack_overflow() {
        let error = ItemError::stack_overflow(10);
        assert_eq!(error.message(), "Stack overflow: maximum stack size is 10");
    }

    #[rstest]
    fn message_invalid_item_quantity() {
        let error = ItemError::invalid_item_quantity();
        assert_eq!(error.message(), "Invalid item quantity");
    }

    // =========================================================================
    // is_not_found Tests
    // =========================================================================

    #[rstest]
    fn is_not_found_true() {
        let error = ItemError::item_not_found("test");
        assert!(error.is_not_found());
    }

    #[rstest]
    fn is_not_found_false() {
        assert!(!ItemError::item_not_usable("test").is_not_found());
        assert!(!ItemError::item_not_equippable("test").is_not_found());
        assert!(!ItemError::stack_overflow(10).is_not_found());
        assert!(!ItemError::invalid_item_quantity().is_not_found());
    }

    // =========================================================================
    // is_usage_error Tests
    // =========================================================================

    #[rstest]
    fn is_usage_error_not_usable() {
        let error = ItemError::item_not_usable("test");
        assert!(error.is_usage_error());
    }

    #[rstest]
    fn is_usage_error_not_equippable() {
        let error = ItemError::item_not_equippable("test");
        assert!(error.is_usage_error());
    }

    #[rstest]
    fn is_usage_error_false() {
        assert!(!ItemError::item_not_found("test").is_usage_error());
        assert!(!ItemError::stack_overflow(10).is_usage_error());
        assert!(!ItemError::invalid_item_quantity().is_usage_error());
    }

    // =========================================================================
    // is_quantity_error Tests
    // =========================================================================

    #[rstest]
    fn is_quantity_error_stack_overflow() {
        let error = ItemError::stack_overflow(10);
        assert!(error.is_quantity_error());
    }

    #[rstest]
    fn is_quantity_error_invalid_quantity() {
        let error = ItemError::invalid_item_quantity();
        assert!(error.is_quantity_error());
    }

    #[rstest]
    fn is_quantity_error_false() {
        assert!(!ItemError::item_not_found("test").is_quantity_error());
        assert!(!ItemError::item_not_usable("test").is_quantity_error());
        assert!(!ItemError::item_not_equippable("test").is_quantity_error());
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_item_not_found() {
        let error = ItemError::item_not_found("test-item");
        assert_eq!(format!("{}", error), "Item not found: test-item");
    }

    #[rstest]
    fn display_item_not_usable() {
        let error = ItemError::item_not_usable("my-sword");
        assert_eq!(format!("{}", error), "Item cannot be used: my-sword");
    }

    #[rstest]
    fn display_item_not_equippable() {
        let error = ItemError::item_not_equippable("health-potion");
        assert_eq!(
            format!("{}", error),
            "Item cannot be equipped: health-potion"
        );
    }

    #[rstest]
    fn display_stack_overflow() {
        let error = ItemError::stack_overflow(99);
        assert_eq!(
            format!("{}", error),
            "Stack overflow: maximum stack size is 99"
        );
    }

    #[rstest]
    fn display_invalid_item_quantity() {
        let error = ItemError::invalid_item_quantity();
        assert_eq!(format!("{}", error), "Invalid item quantity");
    }

    // =========================================================================
    // Equality Tests
    // =========================================================================

    #[rstest]
    fn equality_same_error() {
        let error1 = ItemError::item_not_found("test");
        let error2 = ItemError::item_not_found("test");
        assert_eq!(error1, error2);
    }

    #[rstest]
    fn equality_different_identifier() {
        let error1 = ItemError::item_not_found("item-1");
        let error2 = ItemError::item_not_found("item-2");
        assert_ne!(error1, error2);
    }

    #[rstest]
    fn equality_different_variant() {
        let error1 = ItemError::item_not_found("test");
        let error2 = ItemError::item_not_usable("test");
        assert_ne!(error1, error2);
    }

    #[rstest]
    fn equality_stack_overflow() {
        let error1 = ItemError::stack_overflow(10);
        let error2 = ItemError::stack_overflow(10);
        let error3 = ItemError::stack_overflow(20);

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[rstest]
    fn equality_invalid_item_quantity() {
        let error1 = ItemError::invalid_item_quantity();
        let error2 = ItemError::invalid_item_quantity();
        assert_eq!(error1, error2);
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[rstest]
    fn clone() {
        let error = ItemError::item_not_found("test");
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    #[rstest]
    fn debug_format() {
        let error = ItemError::item_not_found("test-123");
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("ItemNotFound"));
        assert!(debug_string.contains("test-123"));
    }

    // =========================================================================
    // Error Trait Tests
    // =========================================================================

    #[rstest]
    fn implements_error_trait() {
        fn assert_error<T: Error>() {}
        assert_error::<ItemError>();
    }

    #[rstest]
    fn error_source_is_none() {
        let error = ItemError::item_not_found("test");
        assert!(error.source().is_none());
    }
}
