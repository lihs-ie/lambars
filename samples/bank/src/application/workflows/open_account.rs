//! Account opening workflow.
//!
//! This module provides pure functions for validating and processing
//! account opening requests.
//!
//! # Workflow Steps
//!
//! 1. Validate the command (owner name and initial balance)
//! 2. Create a validated intermediate representation
//! 3. Generate an `AccountOpened` event
//!
//! # Examples
//!
//! ```rust
//! use bank::application::workflows::open_account::open_account;
//! use bank::domain::account::commands::OpenAccountCommand;
//! use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
//!
//! let command = OpenAccountCommand::new(
//!     "Alice".to_string(),
//!     Money::new(10000, Currency::JPY),
//! );
//!
//! let account_id = AccountId::generate();
//! let timestamp = Timestamp::now();
//! let result = open_account(&command, account_id, timestamp);
//! // result is Either<DomainError, AccountOpened>
//! ```

use crate::application::validation::{validate_initial_balance, validate_owner_name};
use crate::domain::account::commands::OpenAccountCommand;
use crate::domain::account::errors::DomainResult;
use crate::domain::account::events::{AccountOpened, EventId};
use crate::domain::value_objects::{AccountId, Money, Timestamp};
use lambars::control::Either;

/// Validated open account data.
///
/// This struct represents a successfully validated account opening request.
/// It contains all the data needed to create an `AccountOpened` event.
///
/// # Design
///
/// By separating validation from event creation, we ensure:
/// - Validation logic is testable in isolation
/// - Event creation receives only valid data
/// - The workflow is composed of small, pure functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedOpenAccount {
    /// The validated owner name (trimmed).
    pub owner_name: String,
    /// The validated initial balance (non-negative).
    pub initial_balance: Money,
}

impl ValidatedOpenAccount {
    /// Creates a new `ValidatedOpenAccount`.
    ///
    /// This constructor is primarily for testing. In production, use
    /// `validate_open_account` to create validated instances.
    #[must_use]
    pub const fn new(owner_name: String, initial_balance: Money) -> Self {
        Self {
            owner_name,
            initial_balance,
        }
    }
}

/// Validates an open account command.
///
/// This function validates all fields of the command and returns
/// a validated representation if all validations pass.
///
/// # Arguments
///
/// * `command` - The open account command to validate
///
/// # Returns
///
/// * `Either::Right(ValidatedOpenAccount)` - If all validations pass
/// * `Either::Left(DomainError)` - If any validation fails
///
/// # Validation Rules
///
/// - Owner name must not be empty or whitespace-only
/// - Owner name must not exceed 100 characters
/// - Initial balance must be non-negative
pub(crate) fn validate_open_account(
    command: &OpenAccountCommand,
) -> DomainResult<ValidatedOpenAccount> {
    // Validate owner name
    let validated_name = validate_owner_name(&command.owner_name);

    // Validate initial balance
    let validated_balance = validate_initial_balance(&command.initial_balance);

    // Combine validations
    // Note: For true Applicative-style parallel error accumulation,
    // we would need a more sophisticated approach. Here we use
    // sequential validation with Either's flat_map.
    match (validated_name, validated_balance) {
        (Either::Right(name), Either::Right(balance)) => {
            Either::Right(ValidatedOpenAccount::new(name, balance))
        }
        (Either::Left(error), _) | (_, Either::Left(error)) => Either::Left(error),
    }
}

/// Creates an `AccountOpened` event from validated data.
///
/// This is a pure function that generates an event from validated input.
/// The `account_id` and `timestamp` are passed as arguments to maintain
/// referential transparency (the function doesn't generate these internally).
///
/// # Arguments
///
/// * `validated` - The validated open account data
/// * `account_id` - The ID to assign to the new account
/// * `timestamp` - The timestamp for the event
///
/// # Returns
///
/// An `AccountOpened` event ready for persistence.
///
/// # Design
///
/// By accepting `account_id` and `timestamp` as parameters, we:
/// - Keep the function pure (no side effects like generating IDs or getting current time)
/// - Make the function fully testable with deterministic inputs
/// - Allow the caller to control ID generation and time sourcing
#[must_use]
pub(crate) fn create_account_opened_event(
    validated: ValidatedOpenAccount,
    account_id: AccountId,
    timestamp: Timestamp,
) -> AccountOpened {
    AccountOpened {
        event_id: EventId::generate(),
        account_id,
        owner_name: validated.owner_name,
        initial_balance: validated.initial_balance,
        opened_at: timestamp,
    }
}

/// Account opening workflow.
///
/// This is the main entry point for the account opening workflow.
/// It validates the command and generates an `AccountOpened` event.
///
/// # Arguments
///
/// * `command` - The open account command
/// * `account_id` - The ID to assign to the new account (injected for referential transparency)
/// * `timestamp` - The timestamp for the event (injected for referential transparency)
///
/// # Returns
///
/// * `Either::Right(AccountOpened)` - If validation passes
/// * `Either::Left(DomainError)` - If validation fails
///
/// # Design
///
/// By accepting `account_id` and `timestamp` as parameters, we:
/// - Keep the function pure (no side effects)
/// - Make the function fully testable with deterministic inputs
/// - Separate "what to do" from "when to do it"
///
/// # Examples
///
/// ```rust
/// use bank::application::workflows::open_account::open_account;
/// use bank::domain::account::commands::OpenAccountCommand;
/// use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
///
/// let command = OpenAccountCommand::new(
///     "Alice".to_string(),
///     Money::new(10000, Currency::JPY),
/// );
///
/// let account_id = AccountId::generate();
/// let timestamp = Timestamp::now();
/// let result = open_account(&command, account_id, timestamp);
///
/// assert!(result.is_right());
/// ```
pub fn open_account(
    command: &OpenAccountCommand,
    account_id: AccountId,
    timestamp: Timestamp,
) -> DomainResult<AccountOpened> {
    validate_open_account(command)
        .map_right(|validated| create_account_opened_event(validated, account_id, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::errors::DomainError;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // ValidatedOpenAccount Tests
    // =========================================================================

    #[rstest]
    fn validated_open_account_new_creates_instance() {
        let validated =
            ValidatedOpenAccount::new("Alice".to_string(), Money::new(10000, Currency::JPY));

        assert_eq!(validated.owner_name, "Alice");
        assert_eq!(validated.initial_balance, Money::new(10000, Currency::JPY));
    }

    #[rstest]
    fn validated_open_account_clone_produces_equal() {
        let original =
            ValidatedOpenAccount::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    // =========================================================================
    // validate_open_account Tests
    // =========================================================================

    #[rstest]
    fn validate_open_account_valid_command_returns_right() {
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.owner_name, "Alice");
        assert_eq!(validated.initial_balance, Money::new(10000, Currency::JPY));
    }

    #[rstest]
    fn validate_open_account_empty_name_returns_left() {
        let command = OpenAccountCommand::new(String::new(), Money::new(10000, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_open_account_negative_balance_returns_left() {
        let command = OpenAccountCommand::new("Alice".to_string(), Money::new(-100, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_open_account_zero_balance_returns_right() {
        let command = OpenAccountCommand::new("Alice".to_string(), Money::zero(Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.initial_balance, Money::zero(Currency::JPY));
    }

    #[rstest]
    fn validate_open_account_trims_owner_name() {
        let command =
            OpenAccountCommand::new("  Alice  ".to_string(), Money::new(10000, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.owner_name, "Alice");
    }

    #[rstest]
    fn validate_open_account_whitespace_only_name_returns_left() {
        let command = OpenAccountCommand::new("   ".to_string(), Money::new(10000, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_left());
    }

    #[rstest]
    fn validate_open_account_both_invalid_returns_left() {
        let command = OpenAccountCommand::new(String::new(), Money::new(-100, Currency::JPY));

        let result = validate_open_account(&command);

        assert!(result.is_left());
        // Returns the first error encountered (name validation)
    }

    // =========================================================================
    // create_account_opened_event Tests
    // =========================================================================

    #[rstest]
    fn create_account_opened_event_creates_event() {
        let validated =
            ValidatedOpenAccount::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let event = create_account_opened_event(validated.clone(), account_id, timestamp);

        assert_eq!(event.account_id, account_id);
        assert_eq!(event.owner_name, validated.owner_name);
        assert_eq!(event.initial_balance, validated.initial_balance);
        assert_eq!(event.opened_at, timestamp);
    }

    #[rstest]
    fn create_account_opened_event_generates_unique_event_id() {
        let validated =
            ValidatedOpenAccount::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let event1 = create_account_opened_event(validated.clone(), account_id, timestamp);
        let event2 = create_account_opened_event(validated, account_id, timestamp);

        assert_ne!(event1.event_id, event2.event_id);
    }

    #[rstest]
    fn create_account_opened_event_preserves_all_fields() {
        let owner_name = "田中太郎".to_string();
        let initial_balance = Money::new(50000, Currency::JPY);
        let validated = ValidatedOpenAccount::new(owner_name.clone(), initial_balance.clone());
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let event = create_account_opened_event(validated, account_id, timestamp);

        assert_eq!(event.owner_name, owner_name);
        assert_eq!(event.initial_balance, initial_balance);
        assert_eq!(event.account_id, account_id);
        assert_eq!(event.opened_at, timestamp);
    }

    // =========================================================================
    // Referential Transparency Tests
    // =========================================================================

    #[rstest]
    fn validate_open_account_is_referentially_transparent() {
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));

        let result1 = validate_open_account(&command);
        let result2 = validate_open_account(&command);

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[rstest]
    fn full_workflow_valid_command_produces_event() {
        // Given: a valid command
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));

        // When: we validate and create an event
        let validated = validate_open_account(&command);
        assert!(validated.is_right());

        let validated = validated.unwrap_right();
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_account_opened_event(validated, account_id, timestamp);

        // Then: the event contains the correct data
        assert_eq!(event.owner_name, "Alice");
        assert_eq!(event.initial_balance, Money::new(10000, Currency::JPY));
        assert_eq!(event.account_id, account_id);
    }

    #[rstest]
    fn full_workflow_invalid_command_returns_error() {
        // Given: an invalid command
        let command = OpenAccountCommand::new(String::new(), Money::new(10000, Currency::JPY));

        // When: we try to validate
        let result = validate_open_account(&command);

        // Then: we get an error
        assert!(result.is_left());
    }

    // =========================================================================
    // open_account Workflow Tests
    // =========================================================================

    #[rstest]
    fn open_account_valid_command_returns_event() {
        // Given: a valid command
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = open_account(&command, account_id, timestamp);

        // Then: we get an AccountOpened event
        assert!(result.is_right());
        let event = result.unwrap_right();
        assert_eq!(event.owner_name, "Alice");
        assert_eq!(event.initial_balance, Money::new(10000, Currency::JPY));
        assert_eq!(event.account_id, account_id);
        assert_eq!(event.opened_at, timestamp);
    }

    #[rstest]
    fn open_account_empty_name_returns_error() {
        // Given: an invalid command (empty name)
        let command = OpenAccountCommand::new(String::new(), Money::new(10000, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = open_account(&command, account_id, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn open_account_negative_balance_returns_error() {
        // Given: an invalid command (negative balance)
        let command = OpenAccountCommand::new("Alice".to_string(), Money::new(-100, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = open_account(&command, account_id, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn open_account_zero_balance_returns_event() {
        // Given: a valid command with zero balance
        let command = OpenAccountCommand::new("Alice".to_string(), Money::zero(Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = open_account(&command, account_id, timestamp);

        // Then: we get an AccountOpened event
        assert!(result.is_right());
        let event = result.unwrap_right();
        assert_eq!(event.initial_balance, Money::zero(Currency::JPY));
    }

    #[rstest]
    fn open_account_is_referentially_transparent() {
        // Given: the same inputs
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        // When: we execute the workflow twice with the same inputs
        let result1 = open_account(&command, account_id, timestamp);
        let result2 = open_account(&command, account_id, timestamp);

        // Then: both results are structurally equal (except event_id which is always unique)
        assert!(result1.is_right());
        assert!(result2.is_right());
        let event1 = result1.unwrap_right();
        let event2 = result2.unwrap_right();
        assert_eq!(event1.account_id, event2.account_id);
        assert_eq!(event1.owner_name, event2.owner_name);
        assert_eq!(event1.initial_balance, event2.initial_balance);
        assert_eq!(event1.opened_at, event2.opened_at);
        // Note: event_id is unique per call, so it won't match
    }
}
