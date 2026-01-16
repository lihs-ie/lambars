//! Deposit workflow.
//!
//! This module provides pure functions for validating and processing
//! deposit requests.
//!
//! # Workflow Steps
//!
//! 1. Validate the deposit command (amount must be positive)
//! 2. Validate the account state (must be open for deposits)
//! 3. Calculate the new balance
//! 4. Generate a `MoneyDeposited` event
//!
//! # Examples
//!
//! ```rust
//! use bank::application::workflows::deposit::deposit;
//! use bank::domain::account::commands::DepositCommand;
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId, Timestamp};
//!
//! let account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     balance: Money::new(10000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! let command = DepositCommand::new(
//!     account.id,
//!     Money::new(5000, Currency::JPY),
//!     TransactionId::generate(),
//! );
//!
//! let timestamp = Timestamp::now();
//! let result = deposit(command, &account, timestamp);
//! // result is Either<DomainError, MoneyDeposited>
//! ```

use crate::application::validation::validate_amount;
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::DepositCommand;
use crate::domain::account::errors::{DomainError, DomainResult};
use crate::domain::account::events::{EventId, MoneyDeposited};
use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use lambars::control::Either;

/// Validated deposit data.
///
/// This struct represents a successfully validated deposit request.
/// It contains all the data needed to create a `MoneyDeposited` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDeposit {
    /// The account ID receiving the deposit.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The validated deposit amount (positive).
    pub amount: Money,
    /// The balance after the deposit.
    pub balance_after: Money,
}

impl ValidatedDeposit {
    /// Creates a new `ValidatedDeposit`.
    #[must_use]
    pub const fn new(
        account_id: AccountId,
        transaction_id: TransactionId,
        amount: Money,
        balance_after: Money,
    ) -> Self {
        Self {
            account_id,
            transaction_id,
            amount,
            balance_after,
        }
    }
}

/// Validates a deposit command against an account.
///
/// This function validates the deposit amount and checks that the account
/// can accept deposits.
///
/// # Arguments
///
/// * `command` - The deposit command to validate
/// * `account` - The account to deposit into
///
/// # Returns
///
/// * `Either::Right(ValidatedDeposit)` - If all validations pass
/// * `Either::Left(DomainError)` - If any validation fails
///
/// # Validation Rules
///
/// - Amount must be positive (greater than zero)
/// - Account must not be closed
/// - Frozen accounts can still receive deposits
pub(crate) fn validate_deposit(
    command: &DepositCommand,
    account: &Account,
) -> DomainResult<ValidatedDeposit> {
    // Validate that account can accept deposits
    if let Either::Left(error) = account.can_deposit() {
        return Either::Left(error);
    }

    // Validate amount is positive
    let validated_amount = match validate_amount(&command.amount) {
        Either::Right(amount) => amount,
        Either::Left(error) => return Either::Left(error),
    };

    // Calculate new balance
    let balance_after = match account.balance.add(&validated_amount) {
        Either::Right(balance) => balance,
        Either::Left(money_error) => {
            return Either::Left(DomainError::InvalidAmount(format!(
                "Currency mismatch: {money_error}"
            )));
        }
    };

    Either::Right(ValidatedDeposit::new(
        command.account_id,
        command.transaction_id,
        validated_amount,
        balance_after,
    ))
}

/// Creates a `MoneyDeposited` event from validated data.
///
/// This is a pure function that generates an event from validated input.
/// The `timestamp` is passed as an argument to maintain referential transparency.
///
/// # Arguments
///
/// * `validated` - The validated deposit data
/// * `timestamp` - The timestamp for the event
///
/// # Returns
///
/// A `MoneyDeposited` event ready for persistence.
#[must_use]
pub(crate) fn create_deposit_event(
    validated: ValidatedDeposit,
    timestamp: Timestamp,
) -> MoneyDeposited {
    MoneyDeposited {
        event_id: EventId::generate(),
        account_id: validated.account_id,
        transaction_id: validated.transaction_id,
        amount: validated.amount,
        balance_after: validated.balance_after,
        deposited_at: timestamp,
    }
}

/// Deposit workflow.
///
/// This is the main entry point for the deposit workflow.
/// It validates the command against the account and generates a `MoneyDeposited` event.
///
/// # Arguments
///
/// * `command` - The deposit command
/// * `account` - The account to deposit into
/// * `timestamp` - The timestamp for the event (injected for referential transparency)
///
/// # Returns
///
/// * `Either::Right(MoneyDeposited)` - If validation passes
/// * `Either::Left(DomainError)` - If validation fails
///
/// # Design
///
/// By accepting `timestamp` as a parameter, we:
/// - Keep the function pure (no side effects)
/// - Make the function fully testable with deterministic inputs
/// - Separate "what to do" from "when to do it"
///
/// # Examples
///
/// ```rust
/// use bank::application::workflows::deposit::deposit;
/// use bank::domain::account::commands::DepositCommand;
/// use bank::domain::account::aggregate::{Account, AccountStatus};
/// use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId, Timestamp};
///
/// let account = Account {
///     id: AccountId::generate(),
///     owner_name: "Alice".to_string(),
///     balance: Money::new(10000, Currency::JPY),
///     status: AccountStatus::Active,
///     version: 1,
/// };
///
/// let command = DepositCommand::new(
///     account.id,
///     Money::new(5000, Currency::JPY),
///     TransactionId::generate(),
/// );
///
/// let timestamp = Timestamp::now();
/// let result = deposit(command, &account, timestamp);
///
/// assert!(result.is_right());
/// ```
pub fn deposit(
    command: DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> DomainResult<MoneyDeposited> {
    validate_deposit(&command, account)
        .map_right(|validated| create_deposit_event(validated, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::aggregate::AccountStatus;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_active_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Alice".to_string(),
            balance: Money::new(10000, Currency::JPY),
            status: AccountStatus::Active,
            version: 1,
        }
    }

    fn create_deposit_command(account_id: AccountId, amount: Money) -> DepositCommand {
        DepositCommand::new(account_id, amount, TransactionId::generate())
    }

    // =========================================================================
    // ValidatedDeposit Tests
    // =========================================================================

    #[rstest]
    fn validated_deposit_new_creates_instance() {
        let account_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(15000, Currency::JPY);

        let validated = ValidatedDeposit::new(
            account_id,
            transaction_id,
            amount.clone(),
            balance_after.clone(),
        );

        assert_eq!(validated.account_id, account_id);
        assert_eq!(validated.transaction_id, transaction_id);
        assert_eq!(validated.amount, amount);
        assert_eq!(validated.balance_after, balance_after);
    }

    #[rstest]
    fn validated_deposit_clone_produces_equal() {
        let validated = ValidatedDeposit::new(
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(5000, Currency::JPY),
            Money::new(15000, Currency::JPY),
        );
        let cloned = validated.clone();

        assert_eq!(validated, cloned);
    }

    // =========================================================================
    // validate_deposit Tests
    // =========================================================================

    #[rstest]
    fn validate_deposit_valid_command_returns_right() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_deposit(&command, &account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.account_id, account.id);
        assert_eq!(validated.amount, Money::new(5000, Currency::JPY));
        assert_eq!(validated.balance_after, Money::new(15000, Currency::JPY));
    }

    #[rstest]
    fn validate_deposit_zero_amount_returns_left() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::zero(Currency::JPY));

        let result = validate_deposit(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_deposit_negative_amount_returns_left() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(-1000, Currency::JPY));

        let result = validate_deposit(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_deposit_closed_account_returns_left() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_deposit(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn validate_deposit_frozen_account_returns_right() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_deposit(&command, &account);

        // Frozen accounts can still receive deposits
        assert!(result.is_right());
    }

    #[rstest]
    fn validate_deposit_calculates_correct_balance_after() {
        let mut account = create_active_account();
        account.balance = Money::new(7500, Currency::JPY);
        let command = create_deposit_command(account.id, Money::new(2500, Currency::JPY));

        let result = validate_deposit(&command, &account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.balance_after, Money::new(10000, Currency::JPY));
    }

    #[rstest]
    fn validate_deposit_preserves_transaction_id() {
        let account = create_active_account();
        let transaction_id = TransactionId::generate();
        let command =
            DepositCommand::new(account.id, Money::new(5000, Currency::JPY), transaction_id);

        let result = validate_deposit(&command, &account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.transaction_id, transaction_id);
    }

    // =========================================================================
    // create_deposit_event Tests
    // =========================================================================

    #[rstest]
    fn create_deposit_event_creates_event() {
        let account_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(15000, Currency::JPY);
        let validated = ValidatedDeposit::new(
            account_id,
            transaction_id,
            amount.clone(),
            balance_after.clone(),
        );
        let timestamp = Timestamp::now();

        let event = create_deposit_event(validated, timestamp);

        assert_eq!(event.account_id, account_id);
        assert_eq!(event.transaction_id, transaction_id);
        assert_eq!(event.amount, amount);
        assert_eq!(event.balance_after, balance_after);
        assert_eq!(event.deposited_at, timestamp);
    }

    #[rstest]
    fn create_deposit_event_generates_unique_event_id() {
        let validated = ValidatedDeposit::new(
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(5000, Currency::JPY),
            Money::new(15000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        let event1 = create_deposit_event(validated.clone(), timestamp);
        let event2 = create_deposit_event(validated, timestamp);

        assert_ne!(event1.event_id, event2.event_id);
    }

    // =========================================================================
    // Referential Transparency Tests
    // =========================================================================

    #[rstest]
    fn validate_deposit_is_referentially_transparent() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        let result1 = validate_deposit(&command, &account);
        let result2 = validate_deposit(&command, &account);

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[rstest]
    fn full_workflow_valid_deposit_produces_event() {
        // Given: a valid account and deposit command
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        // When: we validate and create an event
        let validated = validate_deposit(&command, &account);
        assert!(validated.is_right());

        let validated = validated.unwrap_right();
        let timestamp = Timestamp::now();
        let event = create_deposit_event(validated, timestamp);

        // Then: the event contains the correct data
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, Money::new(5000, Currency::JPY));
        assert_eq!(event.balance_after, Money::new(15000, Currency::JPY));
    }

    #[rstest]
    fn full_workflow_invalid_deposit_returns_error() {
        // Given: a closed account
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));

        // When: we try to validate
        let result = validate_deposit(&command, &account);

        // Then: we get an error
        assert!(result.is_left());
    }

    // =========================================================================
    // deposit Workflow Tests
    // =========================================================================

    #[rstest]
    fn deposit_valid_command_returns_event() {
        // Given: a valid account and deposit command
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = deposit(command, &account, timestamp);

        // Then: we get a MoneyDeposited event
        assert!(result.is_right());
        let event = result.unwrap_right();
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, Money::new(5000, Currency::JPY));
        assert_eq!(event.balance_after, Money::new(15000, Currency::JPY));
        assert_eq!(event.deposited_at, timestamp);
    }

    #[rstest]
    fn deposit_zero_amount_returns_error() {
        // Given: a zero amount command
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::zero(Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = deposit(command, &account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn deposit_negative_amount_returns_error() {
        // Given: a negative amount command
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(-1000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = deposit(command, &account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn deposit_closed_account_returns_error() {
        // Given: a closed account
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = deposit(command, &account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn deposit_frozen_account_returns_event() {
        // Given: a frozen account (can still receive deposits)
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = deposit(command, &account, timestamp);

        // Then: we get a MoneyDeposited event
        assert!(result.is_right());
    }

    #[rstest]
    fn deposit_is_referentially_transparent() {
        // Given: the same inputs
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow twice with the same inputs
        let result1 = deposit(command.clone(), &account, timestamp);
        let result2 = deposit(command, &account, timestamp);

        // Then: both results are structurally equal (except event_id)
        assert!(result1.is_right());
        assert!(result2.is_right());
        let event1 = result1.unwrap_right();
        let event2 = result2.unwrap_right();
        assert_eq!(event1.account_id, event2.account_id);
        assert_eq!(event1.transaction_id, event2.transaction_id);
        assert_eq!(event1.amount, event2.amount);
        assert_eq!(event1.balance_after, event2.balance_after);
        assert_eq!(event1.deposited_at, event2.deposited_at);
        // Note: event_id is unique per call
    }
}
