//! Audit logging types for financial operations.
//!
//! This module provides types for audit logging using the Writer monad pattern.
//! Audit logs are accumulated during workflow execution and can be persisted
//! for compliance and debugging purposes.
//!
//! # Design
//!
//! - **Immutable**: Audit entries are immutable once created
//! - **Writer Monad**: Logs are accumulated using `Writer<Vec<AuditEntry>, A>`
//! - **Pure Functions**: Log creation is side-effect free
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::domain::audit::{AuditEntry, AuditAction};
//! use lambars::effect::Writer;
//!
//! fn logged_operation() -> Writer<Vec<AuditEntry>, i32> {
//!     Writer::tell(vec![AuditEntry::new(AuditAction::DepositInitiated)])
//!         .then(Writer::pure(42))
//! }
//! ```

use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use serde::{Deserialize, Serialize};

/// An individual audit log entry.
///
/// Each entry records a single action with optional context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// The action being audited.
    pub action: AuditAction,
    /// When the action occurred.
    pub timestamp: Timestamp,
    /// Optional context data.
    pub context: Option<AuditContext>,
}

impl AuditEntry {
    /// Creates a new audit entry with the current timestamp.
    #[must_use]
    pub fn new(action: AuditAction) -> Self {
        Self {
            action,
            timestamp: Timestamp::now(),
            context: None,
        }
    }

    /// Creates a new audit entry with context.
    #[must_use]
    pub fn with_context(action: AuditAction, context: AuditContext) -> Self {
        Self {
            action,
            timestamp: Timestamp::now(),
            context: Some(context),
        }
    }

    /// Creates an audit entry for a specific timestamp.
    #[must_use]
    pub const fn at(action: AuditAction, timestamp: Timestamp) -> Self {
        Self {
            action,
            timestamp,
            context: None,
        }
    }
}

/// The type of action being audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    // Account lifecycle
    /// Account opening was initiated.
    AccountOpenInitiated,
    /// Account opening validation completed.
    AccountOpenValidated,
    /// Account was successfully opened.
    AccountOpened,

    // Deposit operations
    /// Deposit was initiated.
    DepositInitiated,
    /// Deposit validation completed.
    DepositValidated,
    /// Deposit was successfully processed.
    DepositProcessed,

    // Withdrawal operations
    /// Withdrawal was initiated.
    WithdrawInitiated,
    /// Withdrawal validation completed.
    WithdrawValidated,
    /// Funding source was selected.
    FundingSourceSelected,
    /// Withdrawal was successfully processed.
    WithdrawProcessed,

    // Transfer operations
    /// Transfer was initiated.
    TransferInitiated,
    /// Transfer validation completed.
    TransferValidated,
    /// Transfer was successfully processed.
    TransferProcessed,

    // Validation events
    /// Validation failed with error.
    ValidationFailed,
}

/// Context data for audit entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditContext {
    /// Account-related context.
    Account { account_id: AccountId },
    /// Transaction-related context.
    Transaction {
        transaction_id: TransactionId,
        account_id: AccountId,
        amount: Money,
    },
    /// Transfer-related context.
    Transfer {
        transaction_id: TransactionId,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: Money,
    },
    /// Validation error context.
    ValidationError { error_message: String },
}

// =============================================================================
// Helper Functions for Creating Audit Entries
// =============================================================================

/// Creates an audit entry for account open initiation.
#[must_use]
pub fn account_open_initiated() -> AuditEntry {
    AuditEntry::new(AuditAction::AccountOpenInitiated)
}

/// Creates an audit entry for account open validation.
#[must_use]
pub fn account_open_validated() -> AuditEntry {
    AuditEntry::new(AuditAction::AccountOpenValidated)
}

/// Creates an audit entry for successful account opening.
#[must_use]
pub fn account_opened(account_id: AccountId) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::AccountOpened,
        AuditContext::Account { account_id },
    )
}

/// Creates an audit entry for deposit initiation.
#[must_use]
pub fn deposit_initiated(
    transaction_id: TransactionId,
    account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::DepositInitiated,
        AuditContext::Transaction {
            transaction_id,
            account_id,
            amount,
        },
    )
}

/// Creates an audit entry for deposit validation.
#[must_use]
pub fn deposit_validated() -> AuditEntry {
    AuditEntry::new(AuditAction::DepositValidated)
}

/// Creates an audit entry for processed deposit.
#[must_use]
pub fn deposit_processed(
    transaction_id: TransactionId,
    account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::DepositProcessed,
        AuditContext::Transaction {
            transaction_id,
            account_id,
            amount,
        },
    )
}

/// Creates an audit entry for withdrawal initiation.
#[must_use]
pub fn withdraw_initiated(
    transaction_id: TransactionId,
    account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::WithdrawInitiated,
        AuditContext::Transaction {
            transaction_id,
            account_id,
            amount,
        },
    )
}

/// Creates an audit entry for withdrawal validation.
#[must_use]
pub fn withdraw_validated() -> AuditEntry {
    AuditEntry::new(AuditAction::WithdrawValidated)
}

/// Creates an audit entry for funding source selection.
#[must_use]
pub fn funding_source_selected() -> AuditEntry {
    AuditEntry::new(AuditAction::FundingSourceSelected)
}

/// Creates an audit entry for processed withdrawal.
#[must_use]
pub fn withdraw_processed(
    transaction_id: TransactionId,
    account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::WithdrawProcessed,
        AuditContext::Transaction {
            transaction_id,
            account_id,
            amount,
        },
    )
}

/// Creates an audit entry for transfer initiation.
#[must_use]
pub fn transfer_initiated(
    transaction_id: TransactionId,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::TransferInitiated,
        AuditContext::Transfer {
            transaction_id,
            from_account_id,
            to_account_id,
            amount,
        },
    )
}

/// Creates an audit entry for transfer validation.
#[must_use]
pub fn transfer_validated() -> AuditEntry {
    AuditEntry::new(AuditAction::TransferValidated)
}

/// Creates an audit entry for processed transfer.
#[must_use]
pub fn transfer_processed(
    transaction_id: TransactionId,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: Money,
) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::TransferProcessed,
        AuditContext::Transfer {
            transaction_id,
            from_account_id,
            to_account_id,
            amount,
        },
    )
}

/// Creates an audit entry for validation failure.
#[must_use]
pub fn validation_failed(error_message: String) -> AuditEntry {
    AuditEntry::with_context(
        AuditAction::ValidationFailed,
        AuditContext::ValidationError { error_message },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // AuditEntry Tests
    // =========================================================================

    #[rstest]
    fn audit_entry_new_creates_entry_with_action() {
        let entry = AuditEntry::new(AuditAction::DepositInitiated);

        assert_eq!(entry.action, AuditAction::DepositInitiated);
        assert!(entry.context.is_none());
    }

    #[rstest]
    fn audit_entry_with_context_includes_context() {
        let account_id = AccountId::generate();
        let entry = AuditEntry::with_context(
            AuditAction::AccountOpened,
            AuditContext::Account { account_id },
        );

        assert_eq!(entry.action, AuditAction::AccountOpened);
        assert!(entry.context.is_some());
    }

    // =========================================================================
    // Helper Function Tests
    // =========================================================================

    #[rstest]
    fn account_open_initiated_creates_correct_entry() {
        let entry = account_open_initiated();
        assert_eq!(entry.action, AuditAction::AccountOpenInitiated);
    }

    #[rstest]
    fn deposit_initiated_includes_transaction_context() {
        let transaction_id = TransactionId::generate();
        let account_id = AccountId::generate();
        let amount = Money::new(1000, Currency::JPY);

        let entry = deposit_initiated(transaction_id, account_id, amount.clone());

        assert_eq!(entry.action, AuditAction::DepositInitiated);
        match entry.context {
            Some(AuditContext::Transaction {
                transaction_id: tid,
                account_id: aid,
                amount: amt,
            }) => {
                assert_eq!(tid, transaction_id);
                assert_eq!(aid, account_id);
                assert_eq!(amt, amount);
            }
            _ => panic!("Expected Transaction context"),
        }
    }

    #[rstest]
    fn transfer_initiated_includes_transfer_context() {
        let transaction_id = TransactionId::generate();
        let from_account_id = AccountId::generate();
        let to_account_id = AccountId::generate();
        let amount = Money::new(5000, Currency::USD);

        let entry = transfer_initiated(
            transaction_id,
            from_account_id,
            to_account_id,
            amount.clone(),
        );

        assert_eq!(entry.action, AuditAction::TransferInitiated);
        match entry.context {
            Some(AuditContext::Transfer {
                transaction_id: tid,
                from_account_id: from,
                to_account_id: to,
                amount: amt,
            }) => {
                assert_eq!(tid, transaction_id);
                assert_eq!(from, from_account_id);
                assert_eq!(to, to_account_id);
                assert_eq!(amt, amount);
            }
            _ => panic!("Expected Transfer context"),
        }
    }

    #[rstest]
    fn validation_failed_includes_error_message() {
        let entry = validation_failed("Invalid amount".to_string());

        assert_eq!(entry.action, AuditAction::ValidationFailed);
        match entry.context {
            Some(AuditContext::ValidationError { error_message }) => {
                assert_eq!(error_message, "Invalid amount");
            }
            _ => panic!("Expected ValidationError context"),
        }
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn audit_entry_serializes_to_json() {
        let entry = AuditEntry::new(AuditAction::DepositInitiated);
        let json = serde_json::to_string(&entry);
        assert!(json.is_ok());
    }

    #[rstest]
    fn audit_entry_deserializes_from_json() {
        let entry = AuditEntry::new(AuditAction::DepositInitiated);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action, entry.action);
    }
}
