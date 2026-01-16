//! Transfer workflow.
//!
//! This module provides pure functions for validating and processing
//! transfer requests between two accounts.
//!
//! # Workflow Steps
//!
//! 1. Validate the transfer command (amount must be positive, accounts must differ)
//! 2. Validate the source account state (must be active, have sufficient balance)
//! 3. Validate the destination account state (must not be closed)
//! 4. Calculate new balances for both accounts
//! 5. Generate `TransferSent` and `TransferReceived` events
//!
//! # Examples
//!
//! ```rust
//! use bank::application::workflows::transfer::transfer;
//! use bank::domain::account::commands::TransferCommand;
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId, Timestamp};
//!
//! let from_account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     balance: Money::new(10000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! let to_account = Account {
//!     id: AccountId::generate(),
//!     owner_name: "Bob".to_string(),
//!     balance: Money::new(5000, Currency::JPY),
//!     status: AccountStatus::Active,
//!     version: 1,
//! };
//!
//! let command = TransferCommand::new(
//!     from_account.id,
//!     to_account.id,
//!     Money::new(3000, Currency::JPY),
//!     TransactionId::generate(),
//! );
//!
//! let timestamp = Timestamp::now();
//! let result = transfer(command, &from_account, &to_account, timestamp);
//! // result is Either<DomainError, (TransferSent, TransferReceived)>
//! ```

use crate::application::validation::validate_amount;
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::TransferCommand;
use crate::domain::account::errors::{DomainError, DomainResult};
use crate::domain::account::events::{EventId, TransferReceived, TransferSent};
use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use lambars::control::Either;

/// Validated transfer data.
///
/// This struct represents a successfully validated transfer request.
/// It contains all the data needed to create transfer events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTransfer {
    /// The source account ID.
    pub from_account_id: AccountId,
    /// The destination account ID.
    pub to_account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The validated transfer amount.
    pub amount: Money,
    /// The source account balance after the transfer.
    pub from_balance_after: Money,
    /// The destination account balance after the transfer.
    pub to_balance_after: Money,
}

impl ValidatedTransfer {
    /// Creates a new `ValidatedTransfer`.
    #[must_use]
    pub const fn new(
        from_account_id: AccountId,
        to_account_id: AccountId,
        transaction_id: TransactionId,
        amount: Money,
        from_balance_after: Money,
        to_balance_after: Money,
    ) -> Self {
        Self {
            from_account_id,
            to_account_id,
            transaction_id,
            amount,
            from_balance_after,
            to_balance_after,
        }
    }
}

/// Validates a transfer command against both accounts.
///
/// This function validates the transfer amount, checks both account states,
/// and calculates the resulting balances.
///
/// # Arguments
///
/// * `command` - The transfer command to validate
/// * `from_account` - The source account (sender)
/// * `to_account` - The destination account (receiver)
///
/// # Returns
///
/// * `Either::Right(ValidatedTransfer)` - If all validations pass
/// * `Either::Left(DomainError)` - If any validation fails
///
/// # Validation Rules
///
/// - Amount must be positive (greater than zero)
/// - Source and destination accounts must be different
/// - Source account must be active (not closed or frozen)
/// - Source account must have sufficient balance
/// - Destination account must not be closed (frozen is OK for receiving)
pub(crate) fn validate_transfer(
    command: &TransferCommand,
    from_account: &Account,
    to_account: &Account,
) -> DomainResult<ValidatedTransfer> {
    // Validate that source and destination are different
    if command.from_account_id == command.to_account_id {
        return Either::Left(DomainError::InvalidAmount(
            "Cannot transfer to the same account".to_string(),
        ));
    }

    // Validate amount is positive
    let validated_amount = match validate_amount(&command.amount) {
        Either::Right(amount) => amount,
        Either::Left(error) => return Either::Left(error),
    };

    // Validate that source account can withdraw
    if let Either::Left(error) = from_account.can_withdraw(&validated_amount) {
        return Either::Left(error);
    }

    // Validate that destination account can receive deposits
    if let Either::Left(error) = to_account.can_deposit() {
        return Either::Left(error);
    }

    // Calculate new balances
    let from_balance_after = match from_account.balance.subtract(&validated_amount) {
        Either::Right(balance) => balance,
        Either::Left(money_error) => {
            return Either::Left(DomainError::InvalidAmount(format!(
                "Currency mismatch: {money_error}"
            )));
        }
    };

    let to_balance_after = match to_account.balance.add(&validated_amount) {
        Either::Right(balance) => balance,
        Either::Left(money_error) => {
            return Either::Left(DomainError::InvalidAmount(format!(
                "Currency mismatch: {money_error}"
            )));
        }
    };

    Either::Right(ValidatedTransfer::new(
        command.from_account_id,
        command.to_account_id,
        command.transaction_id,
        validated_amount,
        from_balance_after,
        to_balance_after,
    ))
}

/// Creates `TransferSent` and `TransferReceived` events from validated data.
///
/// This is a pure function that generates a pair of events from validated input.
/// The `timestamp` is passed as an argument to maintain referential transparency.
///
/// # Arguments
///
/// * `validated` - The validated transfer data
/// * `timestamp` - The timestamp for both events
///
/// # Returns
///
/// A tuple of (`TransferSent`, `TransferReceived`) events ready for persistence.
///
/// # Design
///
/// Both events share the same transaction ID and timestamp to maintain
/// consistency. They should be persisted atomically to ensure data integrity.
#[must_use]
pub(crate) fn create_transfer_events(
    validated: ValidatedTransfer,
    timestamp: Timestamp,
) -> (TransferSent, TransferReceived) {
    let sent = TransferSent {
        event_id: EventId::generate(),
        account_id: validated.from_account_id,
        transaction_id: validated.transaction_id,
        to_account_id: validated.to_account_id,
        amount: validated.amount.clone(),
        balance_after: validated.from_balance_after,
        sent_at: timestamp,
    };

    let received = TransferReceived {
        event_id: EventId::generate(),
        account_id: validated.to_account_id,
        transaction_id: validated.transaction_id,
        from_account_id: validated.from_account_id,
        amount: validated.amount,
        balance_after: validated.to_balance_after,
        received_at: timestamp,
    };

    (sent, received)
}

/// Transfer workflow.
///
/// This is the main entry point for the transfer workflow.
/// It validates the command against both accounts and generates
/// `TransferSent` and `TransferReceived` events.
///
/// # Arguments
///
/// * `command` - The transfer command
/// * `from_account` - The source account (sender)
/// * `to_account` - The destination account (receiver)
/// * `timestamp` - The timestamp for the events (injected for referential transparency)
///
/// # Returns
///
/// * `Either::Right((TransferSent, TransferReceived))` - If validation passes
/// * `Either::Left(DomainError)` - If validation fails
///
/// # Design
///
/// By accepting `timestamp` as a parameter, we:
/// - Keep the function pure (no side effects)
/// - Make the function fully testable with deterministic inputs
/// - Separate "what to do" from "when to do it"
///
/// Both returned events share the same transaction ID and timestamp,
/// and should be persisted atomically to ensure data integrity.
///
/// # Examples
///
/// ```rust
/// use bank::application::workflows::transfer::transfer;
/// use bank::domain::account::commands::TransferCommand;
/// use bank::domain::account::aggregate::{Account, AccountStatus};
/// use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId, Timestamp};
///
/// let from_account = Account {
///     id: AccountId::generate(),
///     owner_name: "Alice".to_string(),
///     balance: Money::new(10000, Currency::JPY),
///     status: AccountStatus::Active,
///     version: 1,
/// };
///
/// let to_account = Account {
///     id: AccountId::generate(),
///     owner_name: "Bob".to_string(),
///     balance: Money::new(5000, Currency::JPY),
///     status: AccountStatus::Active,
///     version: 1,
/// };
///
/// let command = TransferCommand::new(
///     from_account.id,
///     to_account.id,
///     Money::new(3000, Currency::JPY),
///     TransactionId::generate(),
/// );
///
/// let timestamp = Timestamp::now();
/// let result = transfer(command, &from_account, &to_account, timestamp);
///
/// assert!(result.is_right());
/// ```
pub fn transfer(
    command: TransferCommand,
    from_account: &Account,
    to_account: &Account,
    timestamp: Timestamp,
) -> DomainResult<(TransferSent, TransferReceived)> {
    validate_transfer(&command, from_account, to_account)
        .map_right(|validated| create_transfer_events(validated, timestamp))
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

    fn create_active_account(balance: Money) -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance,
            status: AccountStatus::Active,
            version: 1,
        }
    }

    fn create_transfer_command(
        from_id: AccountId,
        to_id: AccountId,
        amount: Money,
    ) -> TransferCommand {
        TransferCommand::new(from_id, to_id, amount, TransactionId::generate())
    }

    // =========================================================================
    // ValidatedTransfer Tests
    // =========================================================================

    #[rstest]
    fn validated_transfer_new_creates_instance() {
        let from_id = AccountId::generate();
        let to_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(3000, Currency::JPY);
        let from_balance = Money::new(7000, Currency::JPY);
        let to_balance = Money::new(8000, Currency::JPY);

        let validated = ValidatedTransfer::new(
            from_id,
            to_id,
            transaction_id,
            amount.clone(),
            from_balance.clone(),
            to_balance.clone(),
        );

        assert_eq!(validated.from_account_id, from_id);
        assert_eq!(validated.to_account_id, to_id);
        assert_eq!(validated.transaction_id, transaction_id);
        assert_eq!(validated.amount, amount);
        assert_eq!(validated.from_balance_after, from_balance);
        assert_eq!(validated.to_balance_after, to_balance);
    }

    #[rstest]
    fn validated_transfer_clone_produces_equal() {
        let validated = ValidatedTransfer::new(
            AccountId::generate(),
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(3000, Currency::JPY),
            Money::new(7000, Currency::JPY),
            Money::new(8000, Currency::JPY),
        );
        let cloned = validated.clone();

        assert_eq!(validated, cloned);
    }

    // =========================================================================
    // validate_transfer Tests
    // =========================================================================

    #[rstest]
    fn validate_transfer_valid_command_returns_right() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.from_account_id, from_account.id);
        assert_eq!(validated.to_account_id, to_account.id);
        assert_eq!(validated.amount, Money::new(3000, Currency::JPY));
        assert_eq!(
            validated.from_balance_after,
            Money::new(7000, Currency::JPY)
        );
        assert_eq!(validated.to_balance_after, Money::new(8000, Currency::JPY));
    }

    #[rstest]
    fn validate_transfer_same_account_returns_left() {
        let account = create_active_account(Money::new(10000, Currency::JPY));
        let command =
            create_transfer_command(account.id, account.id, Money::new(3000, Currency::JPY));

        let result = validate_transfer(&command, &account, &account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_transfer_zero_amount_returns_left() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command =
            create_transfer_command(from_account.id, to_account.id, Money::zero(Currency::JPY));

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn validate_transfer_negative_amount_returns_left() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(-1000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
    }

    #[rstest]
    fn validate_transfer_insufficient_balance_returns_left() {
        let from_account = create_active_account(Money::new(2000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InsufficientBalance { .. }));
    }

    #[rstest]
    fn validate_transfer_closed_from_account_returns_left() {
        let mut from_account = create_active_account(Money::new(10000, Currency::JPY));
        from_account.status = AccountStatus::Closed;
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn validate_transfer_frozen_from_account_returns_left() {
        let mut from_account = create_active_account(Money::new(10000, Currency::JPY));
        from_account.status = AccountStatus::Frozen;
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountFrozen(_)));
    }

    #[rstest]
    fn validate_transfer_closed_to_account_returns_left() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let mut to_account = create_active_account(Money::new(5000, Currency::JPY));
        to_account.status = AccountStatus::Closed;
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn validate_transfer_frozen_to_account_returns_right() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let mut to_account = create_active_account(Money::new(5000, Currency::JPY));
        to_account.status = AccountStatus::Frozen;
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        // Frozen accounts can still receive transfers
        assert!(result.is_right());
    }

    #[rstest]
    fn validate_transfer_exact_balance_returns_right() {
        let from_account = create_active_account(Money::new(5000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(5000, Currency::JPY),
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.from_balance_after, Money::zero(Currency::JPY));
        assert_eq!(validated.to_balance_after, Money::new(10000, Currency::JPY));
    }

    #[rstest]
    fn validate_transfer_preserves_transaction_id() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let transaction_id = TransactionId::generate();
        let command = TransferCommand::new(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
            transaction_id,
        );

        let result = validate_transfer(&command, &from_account, &to_account);

        assert!(result.is_right());
        let validated = result.unwrap_right();
        assert_eq!(validated.transaction_id, transaction_id);
    }

    // =========================================================================
    // create_transfer_events Tests
    // =========================================================================

    #[rstest]
    fn create_transfer_events_creates_both_events() {
        let from_id = AccountId::generate();
        let to_id = AccountId::generate();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(3000, Currency::JPY);
        let from_balance = Money::new(7000, Currency::JPY);
        let to_balance = Money::new(8000, Currency::JPY);
        let validated = ValidatedTransfer::new(
            from_id,
            to_id,
            transaction_id,
            amount.clone(),
            from_balance.clone(),
            to_balance.clone(),
        );
        let timestamp = Timestamp::now();

        let (sent, received) = create_transfer_events(validated, timestamp);

        // Check TransferSent
        assert_eq!(sent.account_id, from_id);
        assert_eq!(sent.to_account_id, to_id);
        assert_eq!(sent.transaction_id, transaction_id);
        assert_eq!(sent.amount, amount);
        assert_eq!(sent.balance_after, from_balance);
        assert_eq!(sent.sent_at, timestamp);

        // Check TransferReceived
        assert_eq!(received.account_id, to_id);
        assert_eq!(received.from_account_id, from_id);
        assert_eq!(received.transaction_id, transaction_id);
        assert_eq!(received.amount, amount);
        assert_eq!(received.balance_after, to_balance);
        assert_eq!(received.received_at, timestamp);
    }

    #[rstest]
    fn create_transfer_events_generates_unique_event_ids() {
        let validated = ValidatedTransfer::new(
            AccountId::generate(),
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(3000, Currency::JPY),
            Money::new(7000, Currency::JPY),
            Money::new(8000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        let (sent, received) = create_transfer_events(validated, timestamp);

        // Event IDs should be different between sent and received
        assert_ne!(sent.event_id, received.event_id);
    }

    #[rstest]
    fn create_transfer_events_shares_transaction_id() {
        let validated = ValidatedTransfer::new(
            AccountId::generate(),
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(3000, Currency::JPY),
            Money::new(7000, Currency::JPY),
            Money::new(8000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        let (sent, received) = create_transfer_events(validated, timestamp);

        // Both events should share the same transaction ID
        assert_eq!(sent.transaction_id, received.transaction_id);
    }

    #[rstest]
    fn create_transfer_events_shares_timestamp() {
        let validated = ValidatedTransfer::new(
            AccountId::generate(),
            AccountId::generate(),
            TransactionId::generate(),
            Money::new(3000, Currency::JPY),
            Money::new(7000, Currency::JPY),
            Money::new(8000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        let (sent, received) = create_transfer_events(validated, timestamp);

        // Both events should share the same timestamp
        assert_eq!(sent.sent_at, received.received_at);
    }

    // =========================================================================
    // Referential Transparency Tests
    // =========================================================================

    #[rstest]
    fn validate_transfer_is_referentially_transparent() {
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        let result1 = validate_transfer(&command, &from_account, &to_account);
        let result2 = validate_transfer(&command, &from_account, &to_account);

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[rstest]
    fn full_workflow_valid_transfer_produces_events() {
        // Given: two valid accounts and a transfer command
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );

        // When: we validate and create events
        let validated = validate_transfer(&command, &from_account, &to_account);
        assert!(validated.is_right());

        let validated = validated.unwrap_right();
        let timestamp = Timestamp::now();
        let (sent, received) = create_transfer_events(validated, timestamp);

        // Then: both events contain the correct data
        assert_eq!(sent.amount, Money::new(3000, Currency::JPY));
        assert_eq!(sent.balance_after, Money::new(7000, Currency::JPY));
        assert_eq!(received.amount, Money::new(3000, Currency::JPY));
        assert_eq!(received.balance_after, Money::new(8000, Currency::JPY));
    }

    #[rstest]
    fn full_workflow_invalid_transfer_returns_error() {
        // Given: an invalid transfer (insufficient balance)
        let from_account = create_active_account(Money::new(2000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(5000, Currency::JPY),
        );

        // When: we try to validate
        let result = validate_transfer(&command, &from_account, &to_account);

        // Then: we get an error
        assert!(result.is_left());
    }

    // =========================================================================
    // transfer Workflow Tests
    // =========================================================================

    #[rstest]
    fn transfer_valid_command_returns_events() {
        // Given: two valid accounts and a transfer command
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get both events
        assert!(result.is_right());
        let (sent, received) = result.unwrap_right();

        // Check TransferSent
        assert_eq!(sent.account_id, from_account.id);
        assert_eq!(sent.to_account_id, to_account.id);
        assert_eq!(sent.amount, Money::new(3000, Currency::JPY));
        assert_eq!(sent.balance_after, Money::new(7000, Currency::JPY));
        assert_eq!(sent.sent_at, timestamp);

        // Check TransferReceived
        assert_eq!(received.account_id, to_account.id);
        assert_eq!(received.from_account_id, from_account.id);
        assert_eq!(received.amount, Money::new(3000, Currency::JPY));
        assert_eq!(received.balance_after, Money::new(8000, Currency::JPY));
        assert_eq!(received.received_at, timestamp);
    }

    #[rstest]
    fn transfer_same_account_returns_error() {
        // Given: transfer to the same account
        let account = create_active_account(Money::new(10000, Currency::JPY));
        let command =
            create_transfer_command(account.id, account.id, Money::new(3000, Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &account, &account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn transfer_zero_amount_returns_error() {
        // Given: zero amount transfer
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command =
            create_transfer_command(from_account.id, to_account.id, Money::zero(Currency::JPY));
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InvalidAmount(_)));
    }

    #[rstest]
    fn transfer_insufficient_balance_returns_error() {
        // Given: insufficient balance
        let from_account = create_active_account(Money::new(2000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::InsufficientBalance { .. }));
    }

    #[rstest]
    fn transfer_closed_from_account_returns_error() {
        // Given: closed source account
        let mut from_account = create_active_account(Money::new(10000, Currency::JPY));
        from_account.status = AccountStatus::Closed;
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn transfer_frozen_from_account_returns_error() {
        // Given: frozen source account
        let mut from_account = create_active_account(Money::new(10000, Currency::JPY));
        from_account.status = AccountStatus::Frozen;
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountFrozen(_)));
    }

    #[rstest]
    fn transfer_closed_to_account_returns_error() {
        // Given: closed destination account
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let mut to_account = create_active_account(Money::new(5000, Currency::JPY));
        to_account.status = AccountStatus::Closed;
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get an error
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert!(matches!(error, DomainError::AccountClosed(_)));
    }

    #[rstest]
    fn transfer_frozen_to_account_returns_events() {
        // Given: frozen destination account (can still receive transfers)
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let mut to_account = create_active_account(Money::new(5000, Currency::JPY));
        to_account.status = AccountStatus::Frozen;
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get both events
        assert!(result.is_right());
    }

    #[rstest]
    fn transfer_exact_balance_returns_events() {
        // Given: transfer of exact balance
        let from_account = create_active_account(Money::new(5000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(5000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: we get both events with correct balances
        assert!(result.is_right());
        let (sent, received) = result.unwrap_right();
        assert_eq!(sent.balance_after, Money::zero(Currency::JPY));
        assert_eq!(received.balance_after, Money::new(10000, Currency::JPY));
    }

    #[rstest]
    fn transfer_events_share_transaction_id() {
        // Given: valid transfer
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow
        let result = transfer(command, &from_account, &to_account, timestamp);

        // Then: both events share the same transaction ID
        assert!(result.is_right());
        let (sent, received) = result.unwrap_right();
        assert_eq!(sent.transaction_id, received.transaction_id);
    }

    #[rstest]
    fn transfer_is_referentially_transparent() {
        // Given: the same inputs
        let from_account = create_active_account(Money::new(10000, Currency::JPY));
        let to_account = create_active_account(Money::new(5000, Currency::JPY));
        let command = create_transfer_command(
            from_account.id,
            to_account.id,
            Money::new(3000, Currency::JPY),
        );
        let timestamp = Timestamp::now();

        // When: we execute the workflow twice with the same inputs
        let result1 = transfer(command.clone(), &from_account, &to_account, timestamp);
        let result2 = transfer(command, &from_account, &to_account, timestamp);

        // Then: both results are structurally equal (except event_ids)
        assert!(result1.is_right());
        assert!(result2.is_right());
        let (sent1, received1) = result1.unwrap_right();
        let (sent2, received2) = result2.unwrap_right();

        // TransferSent comparison
        assert_eq!(sent1.account_id, sent2.account_id);
        assert_eq!(sent1.to_account_id, sent2.to_account_id);
        assert_eq!(sent1.transaction_id, sent2.transaction_id);
        assert_eq!(sent1.amount, sent2.amount);
        assert_eq!(sent1.balance_after, sent2.balance_after);
        assert_eq!(sent1.sent_at, sent2.sent_at);

        // TransferReceived comparison
        assert_eq!(received1.account_id, received2.account_id);
        assert_eq!(received1.from_account_id, received2.from_account_id);
        assert_eq!(received1.transaction_id, received2.transaction_id);
        assert_eq!(received1.amount, received2.amount);
        assert_eq!(received1.balance_after, received2.balance_after);
        assert_eq!(received1.received_at, received2.received_at);
        // Note: event_ids are unique per call
    }
}
