//! Command definitions for the Account aggregate.
//!
//! Commands represent user intentions to change the state of the system.
//! They are validated and processed by command handlers, which produce
//! domain events as a result.
//!
//! # Design Principles
//!
//! - **Immutability**: Commands are immutable data structures
//! - **Type Safety**: Each command type carries required data
//! - **Idempotency**: Transaction IDs enable idempotent processing
//!
//! # Command Processing Flow
//!
//! ```text
//! Command → Validate → Execute → Event(s)
//! ```
//!
//! # Examples
//!
//! ```rust
//! use bank::domain::account::commands::{OpenAccountCommand, DepositCommand};
//! use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId};
//!
//! // Create an open account command
//! let open_command = OpenAccountCommand {
//!     owner_name: "Alice".to_string(),
//!     initial_balance: Money::new(10000, Currency::JPY),
//! };
//!
//! // Create a deposit command
//! let deposit_command = DepositCommand {
//!     account_id: AccountId::generate(),
//!     amount: Money::new(5000, Currency::JPY),
//!     transaction_id: TransactionId::generate(),
//! };
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{AccountId, Money, TransactionId};

/// Command to open a new bank account.
///
/// This command initiates the creation of a new account with the specified
/// owner name and initial balance.
///
/// # Validation Rules
///
/// - `owner_name` must not be empty
/// - `initial_balance` must be non-negative
///
/// # Resulting Event
///
/// On success, produces an `AccountOpened` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAccountCommand {
    /// The name of the account owner.
    pub owner_name: String,
    /// The initial balance to deposit when opening the account.
    pub initial_balance: Money,
}

impl OpenAccountCommand {
    /// Creates a new `OpenAccountCommand`.
    ///
    /// # Arguments
    ///
    /// * `owner_name` - The name of the account owner
    /// * `initial_balance` - The initial balance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::commands::OpenAccountCommand;
    /// use bank::domain::value_objects::{Money, Currency};
    ///
    /// let command = OpenAccountCommand::new(
    ///     "Alice".to_string(),
    ///     Money::new(10000, Currency::JPY),
    /// );
    /// ```
    #[must_use]
    pub const fn new(owner_name: String, initial_balance: Money) -> Self {
        Self {
            owner_name,
            initial_balance,
        }
    }
}

/// Command to deposit money into an account.
///
/// This command represents a request to add funds to an existing account.
/// The `transaction_id` enables idempotent processing of duplicate requests.
///
/// # Validation Rules
///
/// - Account must exist
/// - Account must not be closed
/// - `amount` must be positive
///
/// # Resulting Event
///
/// On success, produces a `MoneyDeposited` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositCommand {
    /// The target account ID.
    pub account_id: AccountId,
    /// The amount to deposit.
    pub amount: Money,
    /// Unique transaction ID for idempotency.
    pub transaction_id: TransactionId,
}

impl DepositCommand {
    /// Creates a new `DepositCommand`.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The target account
    /// * `amount` - The amount to deposit
    /// * `transaction_id` - Unique ID for idempotency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::commands::DepositCommand;
    /// use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId};
    ///
    /// let command = DepositCommand::new(
    ///     AccountId::generate(),
    ///     Money::new(5000, Currency::JPY),
    ///     TransactionId::generate(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(account_id: AccountId, amount: Money, transaction_id: TransactionId) -> Self {
        Self {
            account_id,
            amount,
            transaction_id,
        }
    }
}

/// Command to withdraw money from an account.
///
/// This command represents a request to remove funds from an existing account.
/// The `transaction_id` enables idempotent processing of duplicate requests.
///
/// # Validation Rules
///
/// - Account must exist
/// - Account must be active (not closed or frozen)
/// - Account must have sufficient balance
/// - `amount` must be positive
///
/// # Resulting Event
///
/// On success, produces a `MoneyWithdrawn` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithdrawCommand {
    /// The source account ID.
    pub account_id: AccountId,
    /// The amount to withdraw.
    pub amount: Money,
    /// Unique transaction ID for idempotency.
    pub transaction_id: TransactionId,
}

impl WithdrawCommand {
    /// Creates a new `WithdrawCommand`.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The source account
    /// * `amount` - The amount to withdraw
    /// * `transaction_id` - Unique ID for idempotency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::commands::WithdrawCommand;
    /// use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId};
    ///
    /// let command = WithdrawCommand::new(
    ///     AccountId::generate(),
    ///     Money::new(3000, Currency::JPY),
    ///     TransactionId::generate(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(account_id: AccountId, amount: Money, transaction_id: TransactionId) -> Self {
        Self {
            account_id,
            amount,
            transaction_id,
        }
    }
}

/// Command to transfer money between accounts.
///
/// This command represents a request to move funds from one account to another.
/// The transfer is an atomic operation affecting both accounts.
///
/// # Validation Rules
///
/// - Both accounts must exist
/// - Source account must be active (not closed or frozen)
/// - Source account must have sufficient balance
/// - `amount` must be positive
/// - Source and destination accounts must be different
///
/// # Resulting Events
///
/// On success, produces:
/// - `TransferSent` event on the source account
/// - `TransferReceived` event on the destination account
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferCommand {
    /// The source account ID (funds will be deducted from here).
    pub from_account_id: AccountId,
    /// The destination account ID (funds will be added here).
    pub to_account_id: AccountId,
    /// The amount to transfer.
    pub amount: Money,
    /// Unique transaction ID for idempotency.
    pub transaction_id: TransactionId,
}

impl TransferCommand {
    /// Creates a new `TransferCommand`.
    ///
    /// # Arguments
    ///
    /// * `from_account_id` - The source account
    /// * `to_account_id` - The destination account
    /// * `amount` - The amount to transfer
    /// * `transaction_id` - Unique ID for idempotency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::commands::TransferCommand;
    /// use bank::domain::value_objects::{AccountId, Money, Currency, TransactionId};
    ///
    /// let command = TransferCommand::new(
    ///     AccountId::generate(),
    ///     AccountId::generate(),
    ///     Money::new(2000, Currency::JPY),
    ///     TransactionId::generate(),
    /// );
    /// ```
    #[must_use]
    pub const fn new(
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: Money,
        transaction_id: TransactionId,
    ) -> Self {
        Self {
            from_account_id,
            to_account_id,
            amount,
            transaction_id,
        }
    }
}

/// Command to close an account.
///
/// This command represents a request to permanently close an account.
/// Once closed, the account cannot be reopened or used for transactions.
///
/// # Validation Rules
///
/// - Account must exist
/// - Account must not already be closed
/// - Account balance should be zero (or funds must be transferred first)
///
/// # Resulting Event
///
/// On success, produces an `AccountClosed` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseAccountCommand {
    /// The account ID to close.
    pub account_id: AccountId,
}

impl CloseAccountCommand {
    /// Creates a new `CloseAccountCommand`.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account to close
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::commands::CloseAccountCommand;
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let command = CloseAccountCommand::new(AccountId::generate());
    /// ```
    #[must_use]
    pub const fn new(account_id: AccountId) -> Self {
        Self { account_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // OpenAccountCommand Tests
    // =========================================================================

    #[rstest]
    fn open_account_command_new_creates_command() {
        let owner_name = "Alice".to_string();
        let initial_balance = Money::new(10000, Currency::JPY);

        let command = OpenAccountCommand::new(owner_name.clone(), initial_balance.clone());

        assert_eq!(command.owner_name, owner_name);
        assert_eq!(command.initial_balance, initial_balance);
    }

    #[rstest]
    fn open_account_command_clone_produces_equal() {
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let cloned = command.clone();

        assert_eq!(command, cloned);
    }

    #[rstest]
    fn open_account_command_serialize_deserialize_roundtrip() {
        let command =
            OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: OpenAccountCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    // =========================================================================
    // DepositCommand Tests
    // =========================================================================

    #[rstest]
    fn deposit_command_new_creates_command() {
        let account_id = AccountId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let transaction_id = TransactionId::generate();

        let command = DepositCommand::new(account_id, amount.clone(), transaction_id);

        assert_eq!(command.account_id, account_id);
        assert_eq!(command.amount, amount);
        assert_eq!(command.transaction_id, transaction_id);
    }

    #[rstest]
    fn deposit_command_clone_produces_equal() {
        let command = DepositCommand::new(
            AccountId::generate(),
            Money::new(5000, Currency::JPY),
            TransactionId::generate(),
        );
        let cloned = command.clone();

        assert_eq!(command, cloned);
    }

    #[rstest]
    fn deposit_command_serialize_deserialize_roundtrip() {
        let command = DepositCommand::new(
            AccountId::generate(),
            Money::new(5000, Currency::JPY),
            TransactionId::generate(),
        );
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: DepositCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    // =========================================================================
    // WithdrawCommand Tests
    // =========================================================================

    #[rstest]
    fn withdraw_command_new_creates_command() {
        let account_id = AccountId::generate();
        let amount = Money::new(3000, Currency::JPY);
        let transaction_id = TransactionId::generate();

        let command = WithdrawCommand::new(account_id, amount.clone(), transaction_id);

        assert_eq!(command.account_id, account_id);
        assert_eq!(command.amount, amount);
        assert_eq!(command.transaction_id, transaction_id);
    }

    #[rstest]
    fn withdraw_command_clone_produces_equal() {
        let command = WithdrawCommand::new(
            AccountId::generate(),
            Money::new(3000, Currency::JPY),
            TransactionId::generate(),
        );
        let cloned = command.clone();

        assert_eq!(command, cloned);
    }

    #[rstest]
    fn withdraw_command_serialize_deserialize_roundtrip() {
        let command = WithdrawCommand::new(
            AccountId::generate(),
            Money::new(3000, Currency::JPY),
            TransactionId::generate(),
        );
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: WithdrawCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    // =========================================================================
    // TransferCommand Tests
    // =========================================================================

    #[rstest]
    fn transfer_command_new_creates_command() {
        let from_account_id = AccountId::generate();
        let to_account_id = AccountId::generate();
        let amount = Money::new(2000, Currency::JPY);
        let transaction_id = TransactionId::generate();

        let command = TransferCommand::new(
            from_account_id,
            to_account_id,
            amount.clone(),
            transaction_id,
        );

        assert_eq!(command.from_account_id, from_account_id);
        assert_eq!(command.to_account_id, to_account_id);
        assert_eq!(command.amount, amount);
        assert_eq!(command.transaction_id, transaction_id);
    }

    #[rstest]
    fn transfer_command_clone_produces_equal() {
        let command = TransferCommand::new(
            AccountId::generate(),
            AccountId::generate(),
            Money::new(2000, Currency::JPY),
            TransactionId::generate(),
        );
        let cloned = command.clone();

        assert_eq!(command, cloned);
    }

    #[rstest]
    fn transfer_command_serialize_deserialize_roundtrip() {
        let command = TransferCommand::new(
            AccountId::generate(),
            AccountId::generate(),
            Money::new(2000, Currency::JPY),
            TransactionId::generate(),
        );
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: TransferCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    // =========================================================================
    // CloseAccountCommand Tests
    // =========================================================================

    #[rstest]
    fn close_account_command_new_creates_command() {
        let account_id = AccountId::generate();

        let command = CloseAccountCommand::new(account_id);

        assert_eq!(command.account_id, account_id);
    }

    #[rstest]
    fn close_account_command_clone_produces_equal() {
        let command = CloseAccountCommand::new(AccountId::generate());
        let cloned = command.clone();

        assert_eq!(command, cloned);
    }

    #[rstest]
    fn close_account_command_serialize_deserialize_roundtrip() {
        let command = CloseAccountCommand::new(AccountId::generate());
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: CloseAccountCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    #[rstest]
    fn commands_implement_debug() {
        let open = OpenAccountCommand::new("Alice".to_string(), Money::new(10000, Currency::JPY));
        let deposit = DepositCommand::new(
            AccountId::generate(),
            Money::new(5000, Currency::JPY),
            TransactionId::generate(),
        );
        let withdraw = WithdrawCommand::new(
            AccountId::generate(),
            Money::new(3000, Currency::JPY),
            TransactionId::generate(),
        );
        let transfer = TransferCommand::new(
            AccountId::generate(),
            AccountId::generate(),
            Money::new(2000, Currency::JPY),
            TransactionId::generate(),
        );
        let close = CloseAccountCommand::new(AccountId::generate());

        // Ensure Debug is implemented and doesn't panic
        assert!(!format!("{open:?}").is_empty());
        assert!(!format!("{deposit:?}").is_empty());
        assert!(!format!("{withdraw:?}").is_empty());
        assert!(!format!("{transfer:?}").is_empty());
        assert!(!format!("{close:?}").is_empty());
    }
}
