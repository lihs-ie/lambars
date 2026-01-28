//! Account ID value object.
//!
//! Provides a strongly-typed identifier for bank accounts using UUID v7 format.
//! UUID v7 is time-ordered, which is beneficial for database indexing and
//! chronological ordering of accounts.

use std::fmt;
use std::str::FromStr;

use lambars::control::Either;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Validation errors for `AccountId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The provided string is not a valid UUID format.
    InvalidUuidFormat(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuidFormat(value) => {
                write!(formatter, "Invalid UUID format: {value}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// A unique identifier for a bank account.
///
/// `AccountId` uses UUID v7 format, which is time-ordered and suitable for
/// database primary keys. It provides:
///
/// - **Type safety**: Prevents accidental mixing of different ID types
/// - **Smart constructor**: Validates input before construction
/// - **Time ordering**: UUID v7 is chronologically sortable
///
/// # Examples
///
/// ```rust
/// use bank::domain::value_objects::AccountId;
///
/// // Generate a new account ID
/// let id = AccountId::generate();
///
/// // Create from a string (validated)
/// let id_result = AccountId::create("01234567-89ab-cdef-0123-456789abcdef");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Creates a new `AccountId` from a string representation.
    ///
    /// This is a smart constructor that validates the input string is a valid UUID.
    /// Returns `Either::Left(ValidationError)` if the string is not a valid UUID format.
    ///
    /// # Arguments
    ///
    /// * `value` - A string that should be a valid UUID
    ///
    /// # Returns
    ///
    /// * `Either::Right(AccountId)` if the string is a valid UUID
    /// * `Either::Left(ValidationError)` if the string is not a valid UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::AccountId;
    /// use lambars::control::Either;
    ///
    /// let valid = AccountId::create("01234567-89ab-cdef-0123-456789abcdef");
    /// assert!(valid.is_right());
    ///
    /// let invalid = AccountId::create("not-a-uuid");
    /// assert!(invalid.is_left());
    /// ```
    pub fn create(value: &str) -> Either<ValidationError, Self> {
        Uuid::from_str(value).map_or_else(
            |_| Either::Left(ValidationError::InvalidUuidFormat(value.to_string())),
            |uuid| Either::Right(Self(uuid)),
        )
    }

    /// Generates a new `AccountId` using UUID v7.
    ///
    /// UUID v7 is time-ordered, meaning IDs generated later will sort after
    /// IDs generated earlier. This is beneficial for:
    ///
    /// - Database index performance
    /// - Natural chronological ordering
    /// - Reduced index fragmentation
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let id1 = AccountId::generate();
    /// let id2 = AccountId::generate();
    ///
    /// // IDs are unique
    /// assert_ne!(id1, id2);
    /// ```
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    /// Returns the underlying UUID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let id = AccountId::generate();
    /// let uuid = id.as_uuid();
    /// ```
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<Uuid> for AccountId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // AccountId::create Tests
    // =========================================================================

    #[rstest]
    fn create_with_valid_uuid_returns_right() {
        let valid_uuid = "01234567-89ab-cdef-0123-456789abcdef";
        let result = AccountId::create(valid_uuid);

        assert!(result.is_right());
        let account_id = result.unwrap_right();
        assert_eq!(account_id.to_string(), valid_uuid);
    }

    #[rstest]
    fn create_with_invalid_uuid_returns_left() {
        let invalid_uuid = "not-a-valid-uuid";
        let result = AccountId::create(invalid_uuid);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            ValidationError::InvalidUuidFormat(invalid_uuid.to_string())
        );
    }

    #[rstest]
    fn create_with_empty_string_returns_left() {
        let result = AccountId::create("");

        assert!(result.is_left());
    }

    #[rstest]
    fn create_with_uppercase_uuid_returns_right() {
        let uppercase_uuid = "01234567-89AB-CDEF-0123-456789ABCDEF";
        let result = AccountId::create(uppercase_uuid);

        assert!(result.is_right());
    }

    #[rstest]
    fn create_with_uuid_without_hyphens_returns_right() {
        let no_hyphens = "0123456789abcdef0123456789abcdef";
        let result = AccountId::create(no_hyphens);

        assert!(result.is_right());
    }

    // =========================================================================
    // AccountId::generate Tests
    // =========================================================================

    #[rstest]
    fn generate_returns_unique_ids() {
        let id1 = AccountId::generate();
        let id2 = AccountId::generate();

        assert_ne!(id1, id2);
    }

    #[rstest]
    fn generate_produces_v7_uuid() {
        let id = AccountId::generate();
        let uuid = id.as_uuid();

        // UUID v7 has version 7 in the version field
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[rstest]
    fn generated_ids_are_time_ordered() {
        let id1 = AccountId::generate();
        // Small delay is not needed as UUID v7 includes sub-millisecond precision
        let id2 = AccountId::generate();

        // UUID v7 should be chronologically sortable
        // id1 should be less than or equal to id2
        assert!(id1 <= id2);
    }

    // =========================================================================
    // AccountId::as_uuid Tests
    // =========================================================================

    #[rstest]
    fn as_uuid_returns_inner_uuid() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let account_id = AccountId::create(uuid_str).unwrap_right();
        let expected_uuid = Uuid::from_str(uuid_str).unwrap();

        assert_eq!(*account_id.as_uuid(), expected_uuid);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_formats_correctly() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let account_id = AccountId::create(uuid_str).unwrap_right();

        assert_eq!(format!("{account_id}"), uuid_str);
    }

    // =========================================================================
    // ValidationError Tests
    // =========================================================================

    #[rstest]
    fn validation_error_display() {
        let error = ValidationError::InvalidUuidFormat("bad-uuid".to_string());

        assert_eq!(format!("{error}"), "Invalid UUID format: bad-uuid");
    }

    // =========================================================================
    // From<Uuid> Tests
    // =========================================================================

    #[rstest]
    fn from_uuid_creates_account_id() {
        let uuid = Uuid::now_v7();
        let account_id: AccountId = uuid.into();

        assert_eq!(*account_id.as_uuid(), uuid);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_roundtrip() {
        let original = AccountId::generate();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AccountId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn serializes_as_uuid_string() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let account_id = AccountId::create(uuid_str).unwrap_right();
        let serialized = serde_json::to_string(&account_id).unwrap();

        assert_eq!(serialized, format!("\"{uuid_str}\""));
    }

    // =========================================================================
    // Trait Implementation Tests
    // =========================================================================

    #[rstest]
    fn clone_produces_equal_value() {
        let original = AccountId::generate();
        let cloned = original;

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn hash_is_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id = AccountId::generate();

        let mut hasher1 = DefaultHasher::new();
        id.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        id.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[rstest]
    fn ord_is_consistent_with_eq() {
        let id1 = AccountId::generate();
        let id2 = id1; // Same ID

        assert_eq!(id1.cmp(&id2), std::cmp::Ordering::Equal);
        assert_eq!(id1, id2);
    }
}
