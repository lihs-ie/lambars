//! Transaction ID value object.
//!
//! Provides a strongly-typed identifier for transactions with support for
//! idempotency keys. Uses UUID v7 format for time-ordered identifiers.

use std::fmt;
use std::str::FromStr;

use lambars::control::Either;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Validation errors for `TransactionId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionIdError {
    /// The provided string is not a valid UUID format.
    InvalidUuidFormat(String),
}

impl fmt::Display for TransactionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuidFormat(value) => {
                write!(formatter, "Invalid UUID format for transaction ID: {value}")
            }
        }
    }
}

impl std::error::Error for TransactionIdError {}

/// A unique identifier for a transaction.
///
/// `TransactionId` is used to uniquely identify transactions and support
/// idempotency in transaction processing. It provides:
///
/// - **Type safety**: Prevents accidental mixing of different ID types
/// - **Idempotency support**: Can be derived from client-provided idempotency keys
/// - **Time ordering**: Uses UUID v7 for chronological sortability
///
/// # Idempotency
///
/// When a client provides an idempotency key, it can be converted to a
/// deterministic `TransactionId` using `from_idempotency_key`. This ensures
/// that retried requests with the same idempotency key will reference the
/// same transaction.
///
/// # Examples
///
/// ```rust
/// use bank::domain::value_objects::TransactionId;
///
/// // Generate a new transaction ID
/// let id = TransactionId::generate();
///
/// // Create from an idempotency key
/// let idempotent_id = TransactionId::from_idempotency_key("user-123-deposit-001");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransactionId(Uuid);

impl TransactionId {
    /// Creates a new `TransactionId` from a string representation.
    ///
    /// This is a smart constructor that validates the input string is a valid UUID.
    /// Returns `Either::Left(TransactionIdError)` if the string is not a valid UUID format.
    ///
    /// # Arguments
    ///
    /// * `value` - A string that should be a valid UUID
    ///
    /// # Returns
    ///
    /// * `Either::Right(TransactionId)` if the string is a valid UUID
    /// * `Either::Left(TransactionIdError)` if the string is not a valid UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::TransactionId;
    ///
    /// let valid = TransactionId::create("01234567-89ab-cdef-0123-456789abcdef");
    /// assert!(valid.is_right());
    ///
    /// let invalid = TransactionId::create("not-a-uuid");
    /// assert!(invalid.is_left());
    /// ```
    pub fn create(value: &str) -> Either<TransactionIdError, Self> {
        Uuid::from_str(value).map_or_else(
            |_| Either::Left(TransactionIdError::InvalidUuidFormat(value.to_string())),
            |uuid| Either::Right(Self(uuid)),
        )
    }

    /// Generates a new `TransactionId` using UUID v7.
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
    /// use bank::domain::value_objects::TransactionId;
    ///
    /// let id1 = TransactionId::generate();
    /// let id2 = TransactionId::generate();
    ///
    /// // IDs are unique
    /// assert_ne!(id1, id2);
    /// ```
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    /// Creates a `TransactionId` from an idempotency key.
    ///
    /// This method uses UUID v5 (namespace + name hashing) to create a
    /// deterministic UUID from the idempotency key. This ensures that
    /// the same idempotency key always produces the same transaction ID.
    ///
    /// # Arguments
    ///
    /// * `idempotency_key` - A string provided by the client to ensure idempotency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::TransactionId;
    ///
    /// let id1 = TransactionId::from_idempotency_key("user-123-deposit-001");
    /// let id2 = TransactionId::from_idempotency_key("user-123-deposit-001");
    ///
    /// // Same idempotency key produces the same ID
    /// assert_eq!(id1, id2);
    ///
    /// let id3 = TransactionId::from_idempotency_key("user-123-deposit-002");
    /// // Different key produces different ID
    /// assert_ne!(id1, id3);
    /// ```
    #[must_use]
    pub fn from_idempotency_key(idempotency_key: &str) -> Self {
        // Use a fixed namespace UUID for transaction idempotency keys
        // This ensures consistent hashing across all transaction IDs
        const TRANSACTION_NAMESPACE: Uuid = Uuid::from_bytes([
            0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4,
            0x30, 0xc8,
        ]);

        Self(Uuid::new_v5(
            &TRANSACTION_NAMESPACE,
            idempotency_key.as_bytes(),
        ))
    }

    /// Returns the underlying UUID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::value_objects::TransactionId;
    ///
    /// let id = TransactionId::generate();
    /// let uuid = id.as_uuid();
    /// ```
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl From<Uuid> for TransactionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // TransactionId::create Tests
    // =========================================================================

    #[rstest]
    fn create_with_valid_uuid_returns_right() {
        let valid_uuid = "01234567-89ab-cdef-0123-456789abcdef";
        let result = TransactionId::create(valid_uuid);

        assert!(result.is_right());
        let transaction_id = result.unwrap_right();
        assert_eq!(transaction_id.to_string(), valid_uuid);
    }

    #[rstest]
    fn create_with_invalid_uuid_returns_left() {
        let invalid_uuid = "not-a-valid-uuid";
        let result = TransactionId::create(invalid_uuid);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            TransactionIdError::InvalidUuidFormat(invalid_uuid.to_string())
        );
    }

    #[rstest]
    fn create_with_empty_string_returns_left() {
        let result = TransactionId::create("");

        assert!(result.is_left());
    }

    #[rstest]
    fn create_with_uppercase_uuid_returns_right() {
        let uppercase_uuid = "01234567-89AB-CDEF-0123-456789ABCDEF";
        let result = TransactionId::create(uppercase_uuid);

        assert!(result.is_right());
    }

    // =========================================================================
    // TransactionId::generate Tests
    // =========================================================================

    #[rstest]
    fn generate_returns_unique_ids() {
        let id1 = TransactionId::generate();
        let id2 = TransactionId::generate();

        assert_ne!(id1, id2);
    }

    #[rstest]
    fn generate_produces_v7_uuid() {
        let id = TransactionId::generate();
        let uuid = id.as_uuid();

        // UUID v7 has version 7 in the version field
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[rstest]
    fn generated_ids_are_time_ordered() {
        let id1 = TransactionId::generate();
        let id2 = TransactionId::generate();

        // UUID v7 should be chronologically sortable
        assert!(id1 <= id2);
    }

    // =========================================================================
    // TransactionId::from_idempotency_key Tests
    // =========================================================================

    #[rstest]
    fn from_idempotency_key_is_deterministic() {
        let key = "user-123-deposit-001";
        let id1 = TransactionId::from_idempotency_key(key);
        let id2 = TransactionId::from_idempotency_key(key);

        assert_eq!(id1, id2);
    }

    #[rstest]
    fn from_idempotency_key_different_keys_produce_different_ids() {
        let id1 = TransactionId::from_idempotency_key("key-001");
        let id2 = TransactionId::from_idempotency_key("key-002");

        assert_ne!(id1, id2);
    }

    #[rstest]
    fn from_idempotency_key_produces_v5_uuid() {
        let id = TransactionId::from_idempotency_key("test-key");
        let uuid = id.as_uuid();

        // UUID v5 has version 5 in the version field
        assert_eq!(uuid.get_version_num(), 5);
    }

    #[rstest]
    fn from_idempotency_key_empty_string_is_valid() {
        let id = TransactionId::from_idempotency_key("");

        // Empty string is a valid input for idempotency key
        assert!(id.as_uuid().get_version_num() == 5);
    }

    // =========================================================================
    // TransactionId::as_uuid Tests
    // =========================================================================

    #[rstest]
    fn as_uuid_returns_inner_uuid() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let transaction_id = TransactionId::create(uuid_str).unwrap_right();
        let expected_uuid = Uuid::from_str(uuid_str).unwrap();

        assert_eq!(*transaction_id.as_uuid(), expected_uuid);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[rstest]
    fn display_formats_correctly() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let transaction_id = TransactionId::create(uuid_str).unwrap_right();

        assert_eq!(format!("{transaction_id}"), uuid_str);
    }

    // =========================================================================
    // TransactionIdError Tests
    // =========================================================================

    #[rstest]
    fn transaction_id_error_display() {
        let error = TransactionIdError::InvalidUuidFormat("bad-uuid".to_string());

        assert_eq!(
            format!("{error}"),
            "Invalid UUID format for transaction ID: bad-uuid"
        );
    }

    // =========================================================================
    // From<Uuid> Tests
    // =========================================================================

    #[rstest]
    fn from_uuid_creates_transaction_id() {
        let uuid = Uuid::now_v7();
        let transaction_id: TransactionId = uuid.into();

        assert_eq!(*transaction_id.as_uuid(), uuid);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_roundtrip() {
        let original = TransactionId::generate();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: TransactionId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn serializes_as_uuid_string() {
        let uuid_str = "01234567-89ab-cdef-0123-456789abcdef";
        let transaction_id = TransactionId::create(uuid_str).unwrap_right();
        let serialized = serde_json::to_string(&transaction_id).unwrap();

        assert_eq!(serialized, format!("\"{uuid_str}\""));
    }

    // =========================================================================
    // Trait Implementation Tests
    // =========================================================================

    #[rstest]
    fn clone_produces_equal_value() {
        let original = TransactionId::generate();
        let cloned = original;

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn hash_is_consistent() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id = TransactionId::generate();

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
        let id1 = TransactionId::generate();
        let id2 = id1; // Same ID

        assert_eq!(id1.cmp(&id2), std::cmp::Ordering::Equal);
        assert_eq!(id1, id2);
    }
}
