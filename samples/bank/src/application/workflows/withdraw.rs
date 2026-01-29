//! Withdrawal workflow.
//!
//! This module provides pure functions for validating and processing
//! withdrawal requests, including funding source selection.
//!
//! # Workflow Steps
//!
//! 1. Validate the withdrawal command (amount must be positive)
//! 2. Validate the account state (must be active)
//! 3. Select a funding source based on priority
//! 4. Calculate the new balance
//! 5. Generate a `MoneyWithdrawn` event
//!
//! # Funding Source Selection
//!
//! The withdrawal workflow supports multiple funding sources with priority:
//!
//! - `Balance`: Normal account balance (default)
//! - `Overdraft`: Overdraft facility (future)
//! - `CreditLine`: Credit line (future)
//!
//! The selection is a pure function that takes a priority list as an argument,
//! maintaining referential transparency.
//!
//! # Examples
//!
//! ```rust
//! use bank::application::workflows::withdraw::{withdraw, FundingSourceType};
//! use bank::domain::account::commands::WithdrawCommand;
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
//! let command = WithdrawCommand::new(
//!     account.id,
//!     Money::new(5000, Currency::JPY),
//!     TransactionId::generate(),
//! );
//!
//! let funding_priority = vec![FundingSourceType::Balance];
//! let timestamp = Timestamp::now();
//! let result = withdraw(&command, &account, &funding_priority, timestamp);
//! // result is Either<DomainError, MoneyWithdrawn>
//! ```

use crate::application::validation::validate_amount;
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::WithdrawCommand;
use crate::domain::account::errors::{DomainError, DomainResult};
use crate::domain::account::events::{EventId, MoneyWithdrawn};
use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use lambars::control::Either;
use serde::{Deserialize, Serialize};

/// Funding source type for withdrawals.
///
/// Represents the source from which funds are withdrawn.
/// Currently only `Balance` is implemented; others are for future use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingSourceType {
    /// Normal account balance.
    Balance,
    /// Overdraft facility (future implementation).
    Overdraft,
    /// Credit line (future implementation).
    CreditLine,
}

/// Selected funding source with amount.
///
/// Represents a successfully selected funding source and the amount
/// to withdraw from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFunding {
    /// The type of funding source selected.
    pub source_type: FundingSourceType,
    /// The amount to withdraw from this source.
    pub amount: Money,
}

impl SelectedFunding {
    /// Creates a new `SelectedFunding`.
    #[must_use]
    pub const fn new(source_type: FundingSourceType, amount: Money) -> Self {
        Self {
            source_type,
            amount,
        }
    }
}

/// Error when no funding source has sufficient funds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsufficientFundsError {
    /// The amount that was required.
    pub required: Money,
    /// The amount that was available.
    pub available: Money,
}

/// Selects a funding source based on priority and availability.
///
/// This is a pure function that checks each funding source in priority order
/// and returns the first one that has sufficient funds.
///
/// # Arguments
///
/// * `account` - The account to withdraw from
/// * `amount` - The amount to withdraw
/// * `sources` - The priority-ordered list of funding sources to try
///
/// # Returns
///
/// * `Either::Right(SelectedFunding)` - The selected funding source
/// * `Either::Left(InsufficientFundsError)` - If no source has sufficient funds
///
/// # Examples
///
/// ```rust
/// use bank::application::workflows::withdraw::{
///     select_funding_source, FundingSourceType,
/// };
/// use bank::domain::account::aggregate::{Account, AccountStatus};
/// use bank::domain::value_objects::{AccountId, Money, Currency};
///
/// let account = Account {
///     id: AccountId::generate(),
///     owner_name: "Alice".to_string(),
///     balance: Money::new(10000, Currency::JPY),
///     status: AccountStatus::Active,
///     version: 1,
/// };
///
/// let sources = vec![FundingSourceType::Balance, FundingSourceType::Overdraft];
/// let result = select_funding_source(&account, &Money::new(5000, Currency::JPY), &sources);
/// assert!(result.is_right());
/// ```
pub fn select_funding_source(
    account: &Account,
    amount: &Money,
    sources: &[FundingSourceType],
) -> Either<InsufficientFundsError, SelectedFunding> {
    for source in sources {
        match source {
            FundingSourceType::Balance => {
                if account.balance >= *amount {
                    return Either::Right(SelectedFunding::new(
                        FundingSourceType::Balance,
                        amount.clone(),
                    ));
                }
            }
            FundingSourceType::Overdraft | FundingSourceType::CreditLine => {
                // Future implementation: check overdraft limit or credit line
                // For now, these sources are not available
            }
        }
    }

    Either::Left(InsufficientFundsError {
        required: amount.clone(),
        available: account.balance.clone(),
    })
}

/// Validated withdrawal data.
///
/// This struct represents a successfully validated withdrawal request.
/// It contains all the data needed to create a `MoneyWithdrawn` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedWithdraw {
    /// The account ID to withdraw from.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The validated withdrawal amount.
    pub amount: Money,
    /// The selected funding source.
    pub funding_source: FundingSourceType,
    /// The balance after the withdrawal.
    pub balance_after: Money,
}

impl ValidatedWithdraw {
    /// Creates a new `ValidatedWithdraw`.
    #[must_use]
    pub const fn new(
        account_id: AccountId,
        transaction_id: TransactionId,
        amount: Money,
        funding_source: FundingSourceType,
        balance_after: Money,
    ) -> Self {
        Self {
            account_id,
            transaction_id,
            amount,
            funding_source,
            balance_after,
        }
    }
}

/// Validates a withdrawal command against an account.
///
/// This function validates the withdrawal amount, checks account status,
/// and selects an appropriate funding source.
///
/// # Arguments
///
/// * `command` - The withdrawal command to validate
/// * `account` - The account to withdraw from
///
/// # Returns
///
/// * `Either::Right(ValidatedWithdraw)` - If all validations pass
/// * `Either::Left(DomainError)` - If any validation fails
///
/// # Validation Rules
///
/// - Amount must be positive (greater than zero)
/// - Account must be active (not closed or frozen)
/// - Account must have sufficient balance
#[allow(dead_code)]
pub(crate) fn validate_withdraw(
    command: &WithdrawCommand,
    account: &Account,
) -> DomainResult<ValidatedWithdraw> {
    // Validate that account can withdraw
    if let Either::Left(error) = account.can_withdraw(&command.amount) {
        return Either::Left(error);
    }

    // Validate amount is positive
    let validated_amount = match validate_amount(&command.amount) {
        Either::Right(amount) => amount,
        Either::Left(error) => return Either::Left(error),
    };

    // Select funding source (using default priority)
    let funding_sources = [FundingSourceType::Balance];
    let selected = match select_funding_source(account, &validated_amount, &funding_sources) {
        Either::Right(funding) => funding,
        Either::Left(error) => {
            return Either::Left(DomainError::InsufficientBalance {
                required: error.required,
                available: error.available,
            });
        }
    };

    // Calculate new balance
    let balance_after = match account.balance.subtract(&validated_amount) {
        Either::Right(balance) => balance,
        Either::Left(money_error) => {
            return Either::Left(DomainError::InvalidAmount(format!(
                "Currency mismatch: {money_error}"
            )));
        }
    };

    Either::Right(ValidatedWithdraw::new(
        command.account_id,
        command.transaction_id,
        validated_amount,
        selected.source_type,
        balance_after,
    ))
}

/// Validates a withdrawal command with custom funding source priority.
///
/// This version allows the caller to specify the funding source priority,
/// enabling more flexible withdrawal strategies.
///
/// # Arguments
///
/// * `command` - The withdrawal command to validate
/// * `account` - The account to withdraw from
/// * `funding_priority` - The priority-ordered list of funding sources
///
/// # Returns
///
/// * `Either::Right(ValidatedWithdraw)` - If all validations pass
/// * `Either::Left(DomainError)` - If any validation fails
pub(crate) fn validate_withdraw_with_priority(
    command: &WithdrawCommand,
    account: &Account,
    funding_priority: &[FundingSourceType],
) -> DomainResult<ValidatedWithdraw> {
    // Validate that account can withdraw
    if let Either::Left(error) = account.can_withdraw(&command.amount) {
        return Either::Left(error);
    }

    // Validate amount is positive
    let validated_amount = match validate_amount(&command.amount) {
        Either::Right(amount) => amount,
        Either::Left(error) => return Either::Left(error),
    };

    // Select funding source with custom priority
    let selected = match select_funding_source(account, &validated_amount, funding_priority) {
        Either::Right(funding) => funding,
        Either::Left(error) => {
            return Either::Left(DomainError::InsufficientBalance {
                required: error.required,
                available: error.available,
            });
        }
    };

    // Calculate new balance
    let balance_after = match account.balance.subtract(&validated_amount) {
        Either::Right(balance) => balance,
        Either::Left(money_error) => {
            return Either::Left(DomainError::InvalidAmount(format!(
                "Currency mismatch: {money_error}"
            )));
        }
    };

    Either::Right(ValidatedWithdraw::new(
        command.account_id,
        command.transaction_id,
        validated_amount,
        selected.source_type,
        balance_after,
    ))
}

/// Creates a `MoneyWithdrawn` event from validated data.
///
/// This is a pure function that generates an event from validated input.
/// The `timestamp` is passed as an argument to maintain referential transparency.
///
/// # Arguments
///
/// * `validated` - The validated withdrawal data
/// * `timestamp` - The timestamp for the event
///
/// # Returns
///
/// A `MoneyWithdrawn` event ready for persistence.
#[must_use]
pub(crate) fn create_withdraw_event(
    validated: ValidatedWithdraw,
    timestamp: Timestamp,
) -> MoneyWithdrawn {
    MoneyWithdrawn {
        event_id: EventId::generate(),
        account_id: validated.account_id,
        transaction_id: validated.transaction_id,
        amount: validated.amount,
        balance_after: validated.balance_after,
        withdrawn_at: timestamp,
    }
}

/// Withdrawal workflow.
///
/// This is the main entry point for the withdrawal workflow.
/// It validates the command against the account with custom funding source priority
/// and generates a `MoneyWithdrawn` event.
///
/// # Arguments
///
/// * `command` - The withdrawal command
/// * `account` - The account to withdraw from
/// * `funding_priority` - The priority-ordered list of funding sources to try
/// * `timestamp` - The timestamp for the event (injected for referential transparency)
///
/// # Returns
///
/// * `Either::Right(MoneyWithdrawn)` - If validation passes
/// * `Either::Left(DomainError)` - If validation fails
///
/// # Design
///
/// By accepting `funding_priority` and `timestamp` as parameters, we:
/// - Keep the function pure (no side effects)
/// - Make the function fully testable with deterministic inputs
/// - Allow flexible funding source strategies
///
/// # Examples
///
/// ```rust
/// use bank::application::workflows::withdraw::{withdraw, FundingSourceType};
/// use bank::domain::account::commands::WithdrawCommand;
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
/// let command = WithdrawCommand::new(
///     account.id,
///     Money::new(5000, Currency::JPY),
///     TransactionId::generate(),
/// );
///
/// let funding_priority = vec![FundingSourceType::Balance];
/// let timestamp = Timestamp::now();
/// let result = withdraw(&command, &account, &funding_priority, timestamp);
///
/// assert!(result.is_right());
/// ```
pub fn withdraw(
    command: &WithdrawCommand,
    account: &Account,
    funding_priority: &[FundingSourceType],
    timestamp: Timestamp,
) -> DomainResult<MoneyWithdrawn> {
    validate_withdraw_with_priority(command, account, funding_priority)
        .map_right(|validated| create_withdraw_event(validated, timestamp))
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

    fn create_withdraw_command(account_id: AccountId, amount: Money) -> WithdrawCommand {
        WithdrawCommand::new(account_id, amount, TransactionId::generate())
    }

    // =========================================================================
    // FundingSourceType Tests
    // =========================================================================

    #[rstest]
    fn funding_source_type_serialization_roundtrip() {
        let sources = [
            FundingSourceType::Balance,
            FundingSourceType::Overdraft,
            FundingSourceType::CreditLine,
        ];

        for source in sources {
            let serialized = serde_json::to_string(&source).unwrap();
            let deserialized: FundingSourceType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(source, deserialized);
        }
    }

    // =========================================================================
    // SelectedFunding Tests
    // =========================================================================

    #[rstest]
    fn selected_funding_new_creates_instance() {
        let amount = Money::new(5000, Currency::JPY);
        let selected = SelectedFunding::new(FundingSourceType::Balance, amount.clone());

        assert_eq!(selected.source_type, FundingSourceType::Balance);
        assert_eq!(selected.amount, amount);
    }

    // =========================================================================
    // select_funding_source Tests
    // =========================================================================

    #[rstest]
    fn select_funding_source_balance_sufficient_returns_right() {
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);
        let sources = vec![FundingSourceType::Balance];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_right());
        let selected = result.unwrap_right();
        assert_eq!(selected.source_type, FundingSourceType::Balance);
        assert_eq!(selected.amount, amount);
    }

    #[rstest]
    fn select_funding_source_balance_exact_returns_right() {
        let account = create_active_account();
        let amount = Money::new(10000, Currency::JPY);
        let sources = vec![FundingSourceType::Balance];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_right());
    }

    #[rstest]
    fn select_funding_source_balance_insufficient_returns_left() {
        let account = create_active_account();
        let amount = Money::new(15000, Currency::JPY);
        let sources = vec![FundingSourceType::Balance];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(error.required, amount);
        assert_eq!(error.available, account.balance);
    }

    #[rstest]
    fn select_funding_source_empty_sources_returns_left() {
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);
        let sources: Vec<FundingSourceType> = vec![];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_left());
    }

    #[rstest]
    fn select_funding_source_overdraft_only_returns_left() {
        // Overdraft is not implemented yet
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);
        let sources = vec![FundingSourceType::Overdraft];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_left());
    }

    #[rstest]
    fn select_funding_source_priority_selects_first_available() {
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);
        let sources = vec![
            FundingSourceType::Overdraft,  // Not available
            FundingSourceType::Balance,    // Available
            FundingSourceType::CreditLine, // Not checked
        ];

        let result = select_funding_source(&account, &amount, &sources);

        assert!(result.is_right());
        let selected = result.unwrap_right();
        assert_eq!(selected.source_type, FundingSourceType::Balance);
    }

    // =========================================================================
    // ValidatedWithdraw Tests
    // =========================================================================

    #[rstest]
    fn validated_withdraw_new_creates_instance() {
        let account_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(5000, Currency::JPY);

        let validated = ValidatedWithdraw::new(
            account_id,
            transaction_id,
            amount.clone(),
            FundingSourceType::Balance,
            balance_after.clone(),
        );

        assert_eq!(validated.account_id, account_id);
        assert_eq!(validated.transaction_id, transaction_id);
        assert_eq!(validated.amount, amount);
        assert_eq!(validated.funding_source, FundingSourceType::Balance);
        assert_eq!(validated.balance_after, balance_after);
    }

    // =========================================================================
    // validate_withdraw Tests
    // =========================================================================

    #[rstest]
    fn validate_withdraw_valid_command_returns_right() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.account_id, account.id);
        assert_eq!(validated.amount, Money::new(5000, Currency::JPY));
        assert_eq!(validated.balance_after, Money::new(5000, Currency::JPY));
        assert_eq!(validated.funding_source, FundingSourceType::Balance);
    }

    #[rstest]
    fn validate_withdraw_zero_amount_returns_left() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::zero(Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_withdraw_negative_amount_returns_left() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(-1000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_left());
    }

    #[rstest]
    fn validate_withdraw_insufficient_balance_returns_left() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(15000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InsufficientBalance { .. }));
    }

    #[rstest]
    fn validate_withdraw_closed_account_returns_left() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn validate_withdraw_frozen_account_returns_left() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountFrozen(_)));
    }

    #[rstest]
    fn validate_withdraw_exact_balance_returns_right() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(10000, Currency::JPY));

        let result = validate_withdraw(&command, &account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.balance_after, Money::zero(Currency::JPY));
    }

    // =========================================================================
    // validate_withdraw_with_priority Tests
    // =========================================================================

    #[rstest]
    fn validate_withdraw_with_priority_uses_custom_priority() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let priority = vec![FundingSourceType::Balance];

        let result = validate_withdraw_with_priority(&command, &account, &priority);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.funding_source, FundingSourceType::Balance);
    }

    // =========================================================================
    // create_withdraw_event Tests
    // =========================================================================

    #[rstest]
    fn create_withdraw_event_creates_event() {
        let account_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(5000, Currency::JPY);
        let validated = ValidatedWithdraw::new(
            account_id,
            transaction_id,
            amount.clone(),
            FundingSourceType::Balance,
            balance_after.clone(),
        );
        let timestamp = Timestamp::now();

        let event = create_withdraw_event(validated, timestamp);

        assert_eq!(event.account_id, account_id);
        assert_eq!(event.transaction_id, transaction_id);
        assert_eq!(event.amount, amount);
        assert_eq!(event.balance_after, balance_after);
        assert_eq!(event.withdrawn_at, timestamp);
    }

    #[rstest]
    fn create_withdraw_event_generates_unique_event_id() {
        let validated = ValidatedWithdraw::new(
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(5000, Currency::JPY),
            FundingSourceType::Balance,
            Money::new(5000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        let event1 = create_withdraw_event(validated.clone(), timestamp);
        let event2 = create_withdraw_event(validated, timestamp);

        assert_ne!(event1.event_id, event2.event_id);
    }

    // =========================================================================
    // Referential Transparency Tests
    // =========================================================================

    #[rstest]
    fn validate_withdraw_is_referentially_transparent() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));

        let result1 = validate_withdraw(&command, &account);
        let result2 = validate_withdraw(&command, &account);

        assert_eq!(result1, result2);
    }

    #[rstest]
    fn select_funding_source_is_referentially_transparent() {
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);
        let sources = vec![FundingSourceType::Balance];

        let result1 = select_funding_source(&account, &amount, &sources);
        let result2 = select_funding_source(&account, &amount, &sources);

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[rstest]
    fn full_workflow_valid_withdrawal_produces_event() {
        // Given: a valid account and withdrawal command
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(3000, Currency::JPY));

        // When: we validate and create an event
        let validated = validate_withdraw(&command, &account);
        assert!(validated.is_right());

        let validated = validated.unwrap_right();
        let timestamp = Timestamp::now();
        let event = create_withdraw_event(validated, timestamp);

        // Then: the event contains the correct data
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, Money::new(3000, Currency::JPY));
        assert_eq!(event.balance_after, Money::new(7000, Currency::JPY));
    }

    // =========================================================================
    // withdraw Workflow Tests
    // =========================================================================

    #[rstest]
    fn withdraw_valid_command_returns_event() {
        // Given: a valid account and withdrawal command
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get a MoneyWithdrawn event
        assert!(result.is_right());
        let event = result.unwrap_right();
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, Money::new(5000, Currency::JPY));
        assert_eq!(event.balance_after, Money::new(5000, Currency::JPY));
        assert_eq!(event.withdrawn_at, timestamp);
    }

    #[rstest]
    fn withdraw_zero_amount_returns_error() {
        // Given: a zero amount command
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::zero(Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn withdraw_insufficient_balance_returns_error() {
        // Given: insufficient balance
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(15000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InsufficientBalance { .. }));
    }

    #[rstest]
    fn withdraw_closed_account_returns_error() {
        // Given: a closed account
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn withdraw_frozen_account_returns_error() {
        // Given: a frozen account
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountFrozen(_)));
    }

    #[rstest]
    fn withdraw_exact_balance_returns_event() {
        // Given: withdrawal of exact balance
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(10000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: we get a MoneyWithdrawn event with zero balance
        assert!(result.is_right());
        let event = result.unwrap_right();
        assert_eq!(event.balance_after, Money::zero(Currency::JPY));
    }

    #[rstest]
    fn withdraw_is_referentially_transparent() {
        // Given: the same inputs
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = vec![FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        // When: we execute the workflow twice with the same inputs
        let result1 = withdraw(&command, &account, &funding_priority, timestamp);
        let result2 = withdraw(&command, &account, &funding_priority, timestamp);

        // Then: both results are structurally equal (except event_id)
        assert!(result1.is_right());
        assert!(result2.is_right());
        let event1 = result1.unwrap_right();
        let event2 = result2.unwrap_right();
        assert_eq!(event1.account_id, event2.account_id);
        assert_eq!(event1.transaction_id, event2.transaction_id);
        assert_eq!(event1.amount, event2.amount);
        assert_eq!(event1.balance_after, event2.balance_after);
        assert_eq!(event1.withdrawn_at, event2.withdrawn_at);
        // Note: event_id is unique per call
    }
}
