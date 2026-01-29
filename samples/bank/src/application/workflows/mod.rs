//! Workflow modules for the bank application.
//!
//! Workflows are pure functions that compose validation, business logic,
//! and event generation. They follow the pattern:
//!
//! ```text
//! Command → Workflow → Either<DomainError, Event>
//! ```
//!
//! # Main Workflow Functions
//!
//! Each workflow provides a single entry point function:
//!
//! - [`open_account`]: `Command → Either<DomainError, AccountOpened>`
//! - [`deposit`]: `Command + Account → Either<DomainError, MoneyDeposited>`
//! - [`withdraw`]: `Command + Account + FundingPriority → Either<DomainError, MoneyWithdrawn>`
//! - [`transfer`]: `Command + 2 Accounts → Either<DomainError, (TransferSent, TransferReceived)>`
//!
//! # Audited Workflows
//!
//! The [`audited`] module provides workflow functions that produce audit logs
//! alongside their results using the Writer monad pattern:
//!
//! - [`audited::deposit_with_audit`]: Deposit with audit logging
//! - [`audited::withdraw_with_audit`]: Withdraw with audit logging
//!
//! # Design Principles
//!
//! - **Pure Functions**: All workflows are pure (no side effects)
//! - **Either for Errors**: Errors propagate using `Either<DomainError, T>`
//! - **Immutability**: Data flows through transformations without mutation
//! - **Composition**: Workflows are composed from smaller pure functions
//! - **Referential Transparency**: External dependencies (IDs, timestamps) are injected
//! - **Writer Monad**: Audited workflows accumulate logs without side effects

pub mod audited;
pub mod deposit;
pub mod open_account;
pub mod transfer;
pub mod withdraw;

// Re-export main workflow functions
pub use deposit::deposit;
pub use open_account::open_account;
pub use transfer::transfer;
pub use withdraw::{FundingSourceType, withdraw};

// Re-export validated types for advanced use cases
pub use deposit::ValidatedDeposit;
pub use open_account::ValidatedOpenAccount;
pub use transfer::ValidatedTransfer;
pub use withdraw::{InsufficientFundsError, SelectedFunding, ValidatedWithdraw};

// Re-export audited workflow types
pub use audited::{Audited, AuditedResult};
