//! Validation module for the bank application.
//!
//! This module provides validation functions using functional programming patterns.
//! Validations use the `Either` type for error handling and can be composed
//! using Applicative patterns for parallel error accumulation.
//!
//! # Design Principles
//!
//! - **Pure Functions**: All validators are pure functions
//! - **Referential Transparency**: Same input always produces same output
//! - **Composability**: Validators can be combined using Applicative patterns
//!
//! # Examples
//!
//! ```rust
//! use bank::application::validation::{validate_owner_name, validate_amount};
//! use bank::domain::value_objects::{Money, Currency};
//!
//! let name_result = validate_owner_name("Alice");
//! assert!(name_result.is_right());
//!
//! let amount = Money::new(1000, Currency::JPY);
//! let amount_result = validate_amount(&amount);
//! assert!(amount_result.is_right());
//! ```

use crate::domain::account::errors::{DomainError, DomainResult};
use crate::domain::value_objects::Money;
use lambars::control::Either;

/// Maximum allowed length for owner name.
const MAX_OWNER_NAME_LENGTH: usize = 100;

/// Validates an owner name.
///
/// # Validation Rules
///
/// - Must not be empty
/// - Must not exceed 100 characters
/// - Must not be only whitespace
///
/// # Arguments
///
/// * `name` - The owner name to validate
///
/// # Returns
///
/// * `Either::Right(String)` - The trimmed owner name if valid
/// * `Either::Left(DomainError)` - An error if validation fails
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validate_owner_name;
///
/// let valid = validate_owner_name("Alice");
/// assert!(valid.is_right());
///
/// let invalid = validate_owner_name("");
/// assert!(invalid.is_left());
/// ```
pub fn validate_owner_name(name: &str) -> DomainResult<String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Either::Left(DomainError::InvalidAmount(
            "Owner name cannot be empty".to_string(),
        ));
    }

    if trimmed.len() > MAX_OWNER_NAME_LENGTH {
        return Either::Left(DomainError::InvalidAmount(format!(
            "Owner name cannot exceed {MAX_OWNER_NAME_LENGTH} characters"
        )));
    }

    Either::Right(trimmed.to_string())
}

/// Validates a monetary amount.
///
/// # Validation Rules
///
/// - Must be positive (greater than zero)
///
/// # Arguments
///
/// * `amount` - The money amount to validate
///
/// # Returns
///
/// * `Either::Right(Money)` - The validated amount if valid
/// * `Either::Left(DomainError)` - An error if validation fails
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validate_amount;
/// use bank::domain::value_objects::{Money, Currency};
///
/// let valid = Money::new(1000, Currency::JPY);
/// assert!(validate_amount(&valid).is_right());
///
/// let invalid = Money::new(-100, Currency::JPY);
/// assert!(validate_amount(&invalid).is_left());
/// ```
pub fn validate_amount(amount: &Money) -> DomainResult<Money> {
    if amount.is_positive() {
        Either::Right(amount.clone())
    } else if amount.is_zero() {
        Either::Left(DomainError::InvalidAmount(
            "Amount must be greater than zero".to_string(),
        ))
    } else {
        Either::Left(DomainError::InvalidAmount(
            "Amount cannot be negative".to_string(),
        ))
    }
}

/// Validates an initial balance for account opening.
///
/// # Validation Rules
///
/// - Must be non-negative (zero or greater)
///
/// # Arguments
///
/// * `balance` - The initial balance to validate
///
/// # Returns
///
/// * `Either::Right(Money)` - The validated balance if valid
/// * `Either::Left(DomainError)` - An error if validation fails
///
/// # Examples
///
/// ```rust
/// use bank::application::validation::validate_initial_balance;
/// use bank::domain::value_objects::{Money, Currency};
///
/// let valid = Money::new(1000, Currency::JPY);
/// assert!(validate_initial_balance(&valid).is_right());
///
/// let zero = Money::zero(Currency::JPY);
/// assert!(validate_initial_balance(&zero).is_right());
///
/// let invalid = Money::new(-100, Currency::JPY);
/// assert!(validate_initial_balance(&invalid).is_left());
/// ```
pub fn validate_initial_balance(balance: &Money) -> DomainResult<Money> {
    if balance.is_negative() {
        Either::Left(DomainError::InvalidAmount(
            "Initial balance cannot be negative".to_string(),
        ))
    } else {
        Either::Right(balance.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // validate_owner_name Tests
    // =========================================================================

    #[rstest]
    fn validate_owner_name_valid_name_returns_right() {
        let result = validate_owner_name("Alice");

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "Alice");
    }

    #[rstest]
    fn validate_owner_name_with_whitespace_trims() {
        let result = validate_owner_name("  Alice  ");

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "Alice");
    }

    #[rstest]
    fn validate_owner_name_empty_returns_left() {
        let result = validate_owner_name("");

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_owner_name_whitespace_only_returns_left() {
        let result = validate_owner_name("   ");

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_owner_name_too_long_returns_left() {
        let long_name = "a".repeat(101);
        let result = validate_owner_name(&long_name);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_owner_name_at_max_length_returns_right() {
        let max_name = "a".repeat(100);
        let result = validate_owner_name(&max_name);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right().len(), 100);
    }

    #[rstest]
    fn validate_owner_name_unicode_characters_accepted() {
        let result = validate_owner_name("田中太郎");

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), "田中太郎");
    }

    // =========================================================================
    // validate_amount Tests
    // =========================================================================

    #[rstest]
    fn validate_amount_positive_returns_right() {
        let amount = Money::new(1000, Currency::JPY);
        let result = validate_amount(&amount);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), amount);
    }

    #[rstest]
    fn validate_amount_zero_returns_left() {
        let amount = Money::zero(Currency::JPY);
        let result = validate_amount(&amount);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::InvalidAmount(message) = error {
            assert!(message.contains("greater than zero"));
        } else {
            panic!("Expected InvalidAmount error");
        }
    }

    #[rstest]
    fn validate_amount_negative_returns_left() {
        let amount = Money::new(-100, Currency::JPY);
        let result = validate_amount(&amount);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::InvalidAmount(message) = error {
            assert!(message.contains("negative"));
        } else {
            panic!("Expected InvalidAmount error");
        }
    }

    #[rstest]
    fn validate_amount_preserves_currency() {
        let amount = Money::new(1000, Currency::USD);
        let result = validate_amount(&amount);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right().currency(), Currency::USD);
    }

    // =========================================================================
    // validate_initial_balance Tests
    // =========================================================================

    #[rstest]
    fn validate_initial_balance_positive_returns_right() {
        let balance = Money::new(1000, Currency::JPY);
        let result = validate_initial_balance(&balance);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), balance);
    }

    #[rstest]
    fn validate_initial_balance_zero_returns_right() {
        let balance = Money::zero(Currency::JPY);
        let result = validate_initial_balance(&balance);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), balance);
    }

    #[rstest]
    fn validate_initial_balance_negative_returns_left() {
        let balance = Money::new(-100, Currency::JPY);
        let result = validate_initial_balance(&balance);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::InvalidAmount(message) = error {
            assert!(message.contains("negative"));
        } else {
            panic!("Expected InvalidAmount error");
        }
    }

    #[rstest]
    fn validate_initial_balance_preserves_currency() {
        let balance = Money::new(1000, Currency::EUR);
        let result = validate_initial_balance(&balance);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right().currency(), Currency::EUR);
    }

    // =========================================================================
    // Pure Function Tests (Referential Transparency)
    // =========================================================================

    #[rstest]
    fn validators_are_referentially_transparent() {
        let name = "Alice";
        let amount = Money::new(1000, Currency::JPY);

        // Same input should always produce same output
        assert_eq!(validate_owner_name(name), validate_owner_name(name));
        assert_eq!(validate_amount(&amount), validate_amount(&amount));
        assert_eq!(
            validate_initial_balance(&amount),
            validate_initial_balance(&amount)
        );
    }
}
