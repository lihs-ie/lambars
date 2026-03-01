//! Validated-based validators for parallel error accumulation.
//!
//! This module provides validation functions that return `Validated<T>` instead
//! of `Either<DomainError, T>`, enabling parallel error accumulation using
//! Applicative semantics.
//!
//! # Examples
//!
//! ```rust
//! use bank::application::validation::validated_validators::{
//!     validated_owner_name, validated_amount, validated_account_id
//! };
//! use bank::domain::value_objects::{Money, Currency};
//! use lambars::typeclass::Applicative;
//!
//! // Single validation
//! let name_result = validated_owner_name("");
//! assert!(name_result.is_invalid());
//!
//! // Parallel validation with error accumulation
//! let name = validated_owner_name("");
//! let balance = validated_amount(&Money::new(-100, Currency::JPY));
//!
//! let result = name.map2(balance, |n, b| (n, b));
//! // Both errors are accumulated
//! assert_eq!(result.errors().len(), 2);
//! ```

use crate::domain::account::errors::DomainError;
use crate::domain::validation::{Validated, ValidationError};
use crate::domain::value_objects::{AccountId, AccountIdValidationError, Money};

/// Maximum allowed length for owner name.
const MAX_OWNER_NAME_LENGTH: usize = 100;

/// Validates an owner name, returning a `Validated`.
///
/// # Validation Rules
///
/// - Must not be empty
/// - Must not exceed 100 characters
/// - Must not be only whitespace
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validated_validators::validated_owner_name;
///
/// let valid = validated_owner_name("Alice");
/// assert!(valid.is_valid());
///
/// let invalid = validated_owner_name("");
/// assert!(invalid.is_invalid());
/// ```
#[must_use]
pub fn validated_owner_name(name: &str) -> Validated<String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Validated::invalid_with_code("INVALID_OWNER_NAME", "Owner name cannot be empty");
    }

    if trimmed.len() > MAX_OWNER_NAME_LENGTH {
        return Validated::invalid_with_code(
            "INVALID_OWNER_NAME",
            format!("Owner name cannot exceed {MAX_OWNER_NAME_LENGTH} characters"),
        );
    }

    Validated::valid(trimmed.to_string())
}

/// Validates a monetary amount is positive, returning a `Validated`.
///
/// # Validation Rules
///
/// - Must be positive (greater than zero)
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validated_validators::validated_amount;
/// use bank::domain::value_objects::{Money, Currency};
///
/// let valid = Money::new(1000, Currency::JPY);
/// assert!(validated_amount(&valid).is_valid());
///
/// let invalid = Money::new(-100, Currency::JPY);
/// assert!(validated_amount(&invalid).is_invalid());
/// ```
#[must_use]
pub fn validated_amount(amount: &Money) -> Validated<Money> {
    if amount.is_positive() {
        Validated::valid(amount.clone())
    } else if amount.is_zero() {
        Validated::invalid_with_code("INVALID_AMOUNT", "Amount must be greater than zero")
    } else {
        Validated::invalid_with_code("INVALID_AMOUNT", "Amount cannot be negative")
    }
}

/// Validates an initial balance for account opening, returning a `Validated`.
///
/// # Validation Rules
///
/// - Must be non-negative (zero or greater)
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validated_validators::validated_initial_balance;
/// use bank::domain::value_objects::{Money, Currency};
///
/// let valid = Money::new(1000, Currency::JPY);
/// assert!(validated_initial_balance(&valid).is_valid());
///
/// let zero = Money::zero(Currency::JPY);
/// assert!(validated_initial_balance(&zero).is_valid());
///
/// let invalid = Money::new(-100, Currency::JPY);
/// assert!(validated_initial_balance(&invalid).is_invalid());
/// ```
#[must_use]
pub fn validated_initial_balance(balance: &Money) -> Validated<Money> {
    if balance.is_negative() {
        Validated::invalid_with_code("INVALID_AMOUNT", "Initial balance cannot be negative")
    } else {
        Validated::valid(balance.clone())
    }
}

/// Validates an account ID string, returning a `Validated`.
///
/// # Validation Rules
///
/// - Must be a valid UUID format
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validated_validators::validated_account_id;
///
/// let valid = validated_account_id("550e8400-e29b-41d4-a716-446655440000");
/// assert!(valid.is_valid());
///
/// let invalid = validated_account_id("not-a-uuid");
/// assert!(invalid.is_invalid());
/// ```
#[must_use]
pub fn validated_account_id(id_string: &str) -> Validated<AccountId> {
    match AccountId::create(id_string) {
        lambars::control::Either::Right(id) => Validated::valid(id),
        lambars::control::Either::Left(error) => {
            let (code, message) = account_id_error_to_message(error);
            Validated::invalid_with_code(code, message)
        }
    }
}

/// Converts an account ID validation error to an error code and message.
#[must_use]
fn account_id_error_to_message(error: AccountIdValidationError) -> (&'static str, String) {
    match error {
        AccountIdValidationError::InvalidUuidFormat(value) => (
            "INVALID_ACCOUNT_ID",
            format!("Invalid UUID format: {value}"),
        ),
    }
}

/// Converts a domain error to a `ValidationError`.
#[must_use]
pub fn domain_error_to_validation_error(error: DomainError) -> ValidationError {
    match error {
        DomainError::AccountNotFound(id) => {
            ValidationError::new("ACCOUNT_NOT_FOUND", format!("Account not found: {id}"))
        }
        DomainError::AccountClosed(id) => {
            ValidationError::new("ACCOUNT_CLOSED", format!("Account is closed: {id}"))
        }
        DomainError::AccountFrozen(id) => {
            ValidationError::new("ACCOUNT_FROZEN", format!("Account is frozen: {id}"))
        }
        DomainError::InsufficientBalance {
            required,
            available,
        } => ValidationError::new(
            "INSUFFICIENT_BALANCE",
            format!("Insufficient balance: required={required}, available={available}"),
        ),
        DomainError::InvalidAmount(message) => ValidationError::new("INVALID_AMOUNT", message),
        DomainError::ConcurrencyConflict { expected, actual } => ValidationError::new(
            "CONCURRENCY_CONFLICT",
            format!("Concurrency conflict: expected version {expected}, actual version {actual}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use lambars::typeclass::Applicative;
    use rstest::rstest;

    // =========================================================================
    // validated_owner_name Tests
    // =========================================================================

    #[rstest]
    fn validated_owner_name_valid_returns_valid() {
        let result = validated_owner_name("Alice");

        assert!(result.is_valid());
        assert_eq!(result.unwrap(), "Alice");
    }

    #[rstest]
    fn validated_owner_name_empty_returns_invalid() {
        let result = validated_owner_name("");

        assert!(result.is_invalid());
        assert_eq!(result.errors().len(), 1);
        assert_eq!(
            result.errors().iter().next().unwrap().code,
            "INVALID_OWNER_NAME"
        );
    }

    #[rstest]
    fn validated_owner_name_too_long_returns_invalid() {
        let long_name = "a".repeat(101);
        let result = validated_owner_name(&long_name);

        assert!(result.is_invalid());
    }

    // =========================================================================
    // validated_amount Tests
    // =========================================================================

    #[rstest]
    fn validated_amount_positive_returns_valid() {
        let amount = Money::new(1000, Currency::JPY);
        let result = validated_amount(&amount);

        assert!(result.is_valid());
    }

    #[rstest]
    fn validated_amount_zero_returns_invalid() {
        let amount = Money::zero(Currency::JPY);
        let result = validated_amount(&amount);

        assert!(result.is_invalid());
    }

    #[rstest]
    fn validated_amount_negative_returns_invalid() {
        let amount = Money::new(-100, Currency::JPY);
        let result = validated_amount(&amount);

        assert!(result.is_invalid());
    }

    // =========================================================================
    // validated_initial_balance Tests
    // =========================================================================

    #[rstest]
    fn validated_initial_balance_positive_returns_valid() {
        let balance = Money::new(1000, Currency::JPY);
        let result = validated_initial_balance(&balance);

        assert!(result.is_valid());
    }

    #[rstest]
    fn validated_initial_balance_zero_returns_valid() {
        let balance = Money::zero(Currency::JPY);
        let result = validated_initial_balance(&balance);

        assert!(result.is_valid());
    }

    #[rstest]
    fn validated_initial_balance_negative_returns_invalid() {
        let balance = Money::new(-100, Currency::JPY);
        let result = validated_initial_balance(&balance);

        assert!(result.is_invalid());
    }

    // =========================================================================
    // validated_account_id Tests
    // =========================================================================

    #[rstest]
    fn validated_account_id_valid_uuid_returns_valid() {
        let result = validated_account_id("550e8400-e29b-41d4-a716-446655440000");

        assert!(result.is_valid());
    }

    #[rstest]
    fn validated_account_id_invalid_returns_invalid() {
        let result = validated_account_id("not-a-uuid");

        assert!(result.is_invalid());
        assert_eq!(
            result.errors().iter().next().unwrap().code,
            "INVALID_ACCOUNT_ID"
        );
    }

    // =========================================================================
    // Parallel Validation Tests (Applicative Error Accumulation)
    // =========================================================================

    #[rstest]
    fn parallel_validation_accumulates_all_errors() {
        let name = validated_owner_name("");
        let amount = validated_amount(&Money::new(-100, Currency::JPY));

        let result = name.map2(amount, |n, a| (n, a));

        assert!(result.is_invalid());
        // Both errors should be accumulated
        assert_eq!(result.errors().len(), 2);

        let codes: Vec<&str> = result.errors().iter().map(|e| e.code.as_str()).collect();
        assert!(codes.contains(&"INVALID_OWNER_NAME"));
        assert!(codes.contains(&"INVALID_AMOUNT"));
    }

    #[rstest]
    fn parallel_validation_with_all_valid_returns_valid() {
        let name = validated_owner_name("Alice");
        let balance = validated_initial_balance(&Money::new(1000, Currency::JPY));
        let account_id = validated_account_id("550e8400-e29b-41d4-a716-446655440000");

        let result = name.map3(balance, account_id, |n, b, id| (n, b, id));

        assert!(result.is_valid());
    }

    #[rstest]
    fn parallel_validation_with_multiple_invalid_accumulates_all() {
        let name = validated_owner_name("");
        let balance = validated_initial_balance(&Money::new(-100, Currency::JPY));
        let account_id = validated_account_id("invalid");

        let result = name.map3(balance, account_id, |n, b, id| (n, b, id));

        assert!(result.is_invalid());
        // All three errors should be accumulated
        assert_eq!(result.errors().len(), 3);
    }
}
