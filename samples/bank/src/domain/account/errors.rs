//! Domain errors for the Account aggregate.
//!
//! This module defines domain-specific errors that can occur during
//! account operations. All errors are represented as algebraic data types
//! and use lambars' `Either` type for functional error handling.
//!
//! # Design Principles
//!
//! - **Type Safety**: Each error variant carries relevant context
//! - **User Friendly**: `Display` provides clear error messages
//! - **Referential Transparency**: Error handling is pure (no side effects)
//! - **API Integration**: `to_api_error()` enables clean error transformation
//!
//! # Examples
//!
//! ```rust
//! use bank::domain::account::errors::{DomainError, DomainResult};
//! use bank::domain::value_objects::{AccountId, Money, Currency};
//! use lambars::control::Either;
//!
//! fn check_balance(available: Money, required: Money) -> DomainResult<()> {
//!     if available >= required {
//!         Either::Right(())
//!     } else {
//!         Either::Left(DomainError::InsufficientBalance { required, available })
//!     }
//! }
//! ```

use std::fmt;

use lambars::control::Either;
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{AccountId, Money};

/// Domain errors that can occur during account operations.
///
/// Each variant carries context relevant to the error, enabling
/// detailed error messages and appropriate error handling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainError {
    /// The specified account was not found.
    AccountNotFound(AccountId),

    /// The account has insufficient balance for the requested operation.
    InsufficientBalance {
        /// The amount required for the operation.
        required: Money,
        /// The currently available balance.
        available: Money,
    },

    /// The account is closed and cannot accept operations.
    AccountClosed(AccountId),

    /// The account is frozen and operations are temporarily suspended.
    AccountFrozen(AccountId),

    /// The provided amount is invalid for the operation.
    InvalidAmount(String),

    /// A concurrency conflict occurred (optimistic locking failure).
    ConcurrencyConflict {
        /// The expected version when the operation was initiated.
        expected: u64,
        /// The actual version found in the store.
        actual: u64,
    },
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccountNotFound(account_id) => {
                write!(formatter, "Account not found: {account_id}")
            }
            Self::InsufficientBalance {
                required,
                available,
            } => {
                write!(
                    formatter,
                    "Insufficient balance: required {required}, available {available}"
                )
            }
            Self::AccountClosed(account_id) => {
                write!(formatter, "Account is closed: {account_id}")
            }
            Self::AccountFrozen(account_id) => {
                write!(formatter, "Account is frozen: {account_id}")
            }
            Self::InvalidAmount(reason) => {
                write!(formatter, "Invalid amount: {reason}")
            }
            Self::ConcurrencyConflict { expected, actual } => {
                write!(
                    formatter,
                    "Concurrency conflict: expected version {expected}, actual version {actual}"
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// API error representation for transforming domain errors.
///
/// This enum represents HTTP-level error categories that can be
/// returned to API clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// Resource not found (HTTP 404).
    NotFound,
    /// Bad request due to invalid input (HTTP 400).
    BadRequest,
    /// Conflict with current state (HTTP 409).
    Conflict,
    /// Internal server error (HTTP 500).
    InternalError,
}

impl DomainError {
    /// Converts this domain error to an API error kind.
    ///
    /// This method maps domain-specific errors to HTTP-appropriate
    /// error categories for API responses.
    ///
    /// # Returns
    ///
    /// The appropriate `ApiErrorKind` for this domain error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::errors::{DomainError, ApiErrorKind};
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let error = DomainError::AccountNotFound(AccountId::generate());
    /// assert_eq!(error.to_api_error(), ApiErrorKind::NotFound);
    /// ```
    #[must_use]
    pub const fn to_api_error(&self) -> ApiErrorKind {
        match self {
            Self::AccountNotFound(_) => ApiErrorKind::NotFound,
            Self::InsufficientBalance { .. } | Self::InvalidAmount(_) => ApiErrorKind::BadRequest,
            Self::AccountClosed(_) | Self::AccountFrozen(_) | Self::ConcurrencyConflict { .. } => {
                ApiErrorKind::Conflict
            }
        }
    }
}

/// A type alias for domain operation results.
///
/// Uses lambars' `Either` type with `DomainError` as the left (error) type.
/// This follows the functional programming convention where:
/// - `Left` represents failure/error
/// - `Right` represents success/value
///
/// # Examples
///
/// ```rust
/// use bank::domain::account::errors::{DomainError, DomainResult};
/// use lambars::control::Either;
///
/// fn pure_operation() -> DomainResult<i32> {
///     Either::Right(42)
/// }
///
/// fn failing_operation() -> DomainResult<i32> {
///     Either::Left(DomainError::InvalidAmount("negative value".to_string()))
/// }
/// ```
pub type DomainResult<T> = Either<DomainError, T>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // DomainError Construction Tests
    // =========================================================================

    #[rstest]
    fn account_not_found_contains_account_id() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountNotFound(account_id);

        if let DomainError::AccountNotFound(id) = error {
            assert_eq!(id, account_id);
        } else {
            panic!("Expected AccountNotFound variant");
        }
    }

    #[rstest]
    fn insufficient_balance_contains_amounts() {
        let required = Money::new(1000, Currency::JPY);
        let available = Money::new(500, Currency::JPY);
        let error = DomainError::InsufficientBalance {
            required: required.clone(),
            available: available.clone(),
        };

        if let DomainError::InsufficientBalance {
            required: r,
            available: a,
        } = error
        {
            assert_eq!(r, required);
            assert_eq!(a, available);
        } else {
            panic!("Expected InsufficientBalance variant");
        }
    }

    #[rstest]
    fn account_closed_contains_account_id() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountClosed(account_id);

        if let DomainError::AccountClosed(id) = error {
            assert_eq!(id, account_id);
        } else {
            panic!("Expected AccountClosed variant");
        }
    }

    #[rstest]
    fn account_frozen_contains_account_id() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountFrozen(account_id);

        if let DomainError::AccountFrozen(id) = error {
            assert_eq!(id, account_id);
        } else {
            panic!("Expected AccountFrozen variant");
        }
    }

    #[rstest]
    fn invalid_amount_contains_reason() {
        let reason = "negative value not allowed".to_string();
        let error = DomainError::InvalidAmount(reason.clone());

        if let DomainError::InvalidAmount(r) = error {
            assert_eq!(r, reason);
        } else {
            panic!("Expected InvalidAmount variant");
        }
    }

    #[rstest]
    fn concurrency_conflict_contains_versions() {
        let error = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };

        if let DomainError::ConcurrencyConflict { expected, actual } = error {
            assert_eq!(expected, 5);
            assert_eq!(actual, 7);
        } else {
            panic!("Expected ConcurrencyConflict variant");
        }
    }

    // =========================================================================
    // DomainError Display Tests
    // =========================================================================

    #[rstest]
    fn display_account_not_found() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountNotFound(account_id);

        let message = format!("{error}");
        assert!(message.contains("Account not found:"));
        assert!(message.contains(&account_id.to_string()));
    }

    #[rstest]
    fn display_insufficient_balance() {
        let required = Money::new(1000, Currency::JPY);
        let available = Money::new(500, Currency::JPY);
        let error = DomainError::InsufficientBalance {
            required,
            available,
        };

        let message = format!("{error}");
        assert!(message.contains("Insufficient balance:"));
        assert!(message.contains("required"));
        assert!(message.contains("available"));
    }

    #[rstest]
    fn display_account_closed() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountClosed(account_id);

        let message = format!("{error}");
        assert!(message.contains("Account is closed:"));
        assert!(message.contains(&account_id.to_string()));
    }

    #[rstest]
    fn display_account_frozen() {
        let account_id = AccountId::generate();
        let error = DomainError::AccountFrozen(account_id);

        let message = format!("{error}");
        assert!(message.contains("Account is frozen:"));
        assert!(message.contains(&account_id.to_string()));
    }

    #[rstest]
    fn display_invalid_amount() {
        let error = DomainError::InvalidAmount("cannot be negative".to_string());

        let message = format!("{error}");
        assert!(message.contains("Invalid amount:"));
        assert!(message.contains("cannot be negative"));
    }

    #[rstest]
    fn display_concurrency_conflict() {
        let error = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };

        let message = format!("{error}");
        assert!(message.contains("Concurrency conflict:"));
        assert!(message.contains("expected version 5"));
        assert!(message.contains("actual version 7"));
    }

    // =========================================================================
    // DomainError to_api_error Tests
    // =========================================================================

    #[rstest]
    fn to_api_error_account_not_found_returns_not_found() {
        let error = DomainError::AccountNotFound(AccountId::generate());
        assert_eq!(error.to_api_error(), ApiErrorKind::NotFound);
    }

    #[rstest]
    fn to_api_error_insufficient_balance_returns_bad_request() {
        let error = DomainError::InsufficientBalance {
            required: Money::new(1000, Currency::JPY),
            available: Money::new(500, Currency::JPY),
        };
        assert_eq!(error.to_api_error(), ApiErrorKind::BadRequest);
    }

    #[rstest]
    fn to_api_error_account_closed_returns_conflict() {
        let error = DomainError::AccountClosed(AccountId::generate());
        assert_eq!(error.to_api_error(), ApiErrorKind::Conflict);
    }

    #[rstest]
    fn to_api_error_account_frozen_returns_conflict() {
        let error = DomainError::AccountFrozen(AccountId::generate());
        assert_eq!(error.to_api_error(), ApiErrorKind::Conflict);
    }

    #[rstest]
    fn to_api_error_invalid_amount_returns_bad_request() {
        let error = DomainError::InvalidAmount("test".to_string());
        assert_eq!(error.to_api_error(), ApiErrorKind::BadRequest);
    }

    #[rstest]
    fn to_api_error_concurrency_conflict_returns_conflict() {
        let error = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };
        assert_eq!(error.to_api_error(), ApiErrorKind::Conflict);
    }

    // =========================================================================
    // DomainResult Tests
    // =========================================================================

    #[rstest]
    fn domain_result_right_contains_value() {
        let result: DomainResult<i32> = Either::Right(42);
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), 42);
    }

    #[rstest]
    fn domain_result_left_contains_error() {
        let error = DomainError::InvalidAmount("test".to_string());
        let result: DomainResult<i32> = Either::Left(error.clone());
        assert!(result.is_left());
        assert_eq!(result.unwrap_left(), error);
    }

    #[rstest]
    fn domain_result_map_right_on_success() {
        let result: DomainResult<i32> = Either::Right(21);
        let doubled = result.map_right(|x| x * 2);
        assert_eq!(doubled.unwrap_right(), 42);
    }

    #[rstest]
    fn domain_result_map_right_on_failure() {
        let error = DomainError::InvalidAmount("test".to_string());
        let result: DomainResult<i32> = Either::Left(error.clone());
        let doubled = result.map_right(|x| x * 2);
        assert_eq!(doubled.unwrap_left(), error);
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_account_not_found() {
        let original = DomainError::AccountNotFound(AccountId::generate());
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: DomainError = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn serialize_deserialize_insufficient_balance() {
        let original = DomainError::InsufficientBalance {
            required: Money::new(1000, Currency::JPY),
            available: Money::new(500, Currency::JPY),
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: DomainError = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn serialize_deserialize_concurrency_conflict() {
        let original = DomainError::ConcurrencyConflict {
            expected: 5,
            actual: 7,
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: DomainError = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[rstest]
    fn clone_produces_equal_error() {
        let original = DomainError::InsufficientBalance {
            required: Money::new(1000, Currency::JPY),
            available: Money::new(500, Currency::JPY),
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    #[rstest]
    fn debug_format_contains_variant_name() {
        let error = DomainError::InvalidAmount("test".to_string());
        let debug_output = format!("{error:?}");
        assert!(debug_output.contains("InvalidAmount"));
    }
}
