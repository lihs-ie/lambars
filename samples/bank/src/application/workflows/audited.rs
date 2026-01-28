//! Audited workflow functions using the Writer monad.
//!
//! This module provides workflow functions that produce audit logs alongside
//! their results using the Writer monad pattern. Audit entries are accumulated
//! during workflow execution without side effects.
//!
//! # Design
//!
//! Each audited workflow function returns:
//! ```text
//! Writer<Vec<AuditEntry>, DomainResult<Event>>
//! ```
//!
//! This allows:
//! - **Accumulating logs**: Multiple audit entries are collected during execution
//! - **Pure functions**: No side effects; logs are values
//! - **Composition**: Workflows can be chained while preserving logs
//! - **Separation of concerns**: Logging is separate from business logic
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::application::workflows::audited::deposit_with_audit;
//!
//! let writer = deposit_with_audit(&command, &account, timestamp);
//! let (result, audit_logs) = writer.run();
//!
//! // result: Either<DomainError, MoneyDeposited>
//! // audit_logs: Vec<AuditEntry>
//! ```

use crate::application::workflows::deposit::{create_deposit_event, validate_deposit};
use crate::application::workflows::withdraw::{
    FundingSourceType, create_withdraw_event, validate_withdraw_with_priority,
};
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::{DepositCommand, WithdrawCommand};
use crate::domain::account::errors::DomainResult;
use crate::domain::account::events::{MoneyDeposited, MoneyWithdrawn};
use crate::domain::audit::{
    AuditEntry, deposit_initiated, deposit_processed, deposit_validated, funding_source_selected,
    validation_failed, withdraw_initiated, withdraw_processed, withdraw_validated,
};
use crate::domain::value_objects::Timestamp;
use lambars::control::Either;
use lambars::effect::Writer;

/// Type alias for an audited workflow result.
pub type Audited<T> = Writer<Vec<AuditEntry>, T>;

/// Type alias for an audited domain result.
pub type AuditedResult<T> = Audited<DomainResult<T>>;

/// Deposit workflow with audit logging.
///
/// Executes the deposit workflow and accumulates audit entries for each step:
/// 1. `DepositInitiated` - When the deposit process starts
/// 2. `DepositValidated` or `ValidationFailed` - After validation
/// 3. `DepositProcessed` - After successful event creation
///
/// # Arguments
///
/// * `command` - The deposit command to process
/// * `account` - The account to deposit into
/// * `timestamp` - The timestamp for the event
///
/// # Returns
///
/// A `Writer` containing:
/// - The result: `Either<DomainError, MoneyDeposited>`
/// - Accumulated audit entries
///
/// # Example
///
/// ```rust,ignore
/// let writer = deposit_with_audit(&command, &account, timestamp);
/// let (result, logs) = writer.run();
///
/// match result {
///     Either::Right(event) => println!("Deposit processed: {:?}", event),
///     Either::Left(error) => println!("Deposit failed: {:?}", error),
/// }
///
/// for log in logs {
///     println!("Audit: {:?}", log);
/// }
/// ```
pub fn deposit_with_audit(
    command: &DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> AuditedResult<MoneyDeposited> {
    let initiated_log = deposit_initiated(
        command.transaction_id,
        command.account_id,
        command.amount.clone(),
    );

    Writer::tell(vec![initiated_log]).flat_map(|()| {
        let validation_result = validate_deposit(command, account);

        match validation_result {
            Either::Right(validated) => {
                let event = create_deposit_event(validated, timestamp);
                let validated_log = deposit_validated();
                let processed_log =
                    deposit_processed(event.transaction_id, event.account_id, event.amount.clone());

                Writer::new(Either::Right(event), vec![validated_log, processed_log])
            }
            Either::Left(error) => {
                let error_log = validation_failed(format!("{error:?}"));
                Writer::new(Either::Left(error), vec![error_log])
            }
        }
    })
}

/// Withdraw workflow with audit logging.
///
/// Executes the withdraw workflow and accumulates audit entries for each step:
/// 1. `WithdrawInitiated` - When the withdrawal process starts
/// 2. `WithdrawValidated` or `ValidationFailed` - After validation (includes funding source selection)
/// 3. `FundingSourceSelected` - After funding source selection
/// 4. `WithdrawProcessed` - After successful event creation
///
/// # Arguments
///
/// * `command` - The withdraw command to process
/// * `account` - The account to withdraw from
/// * `funding_priority` - The priority order for funding sources
/// * `timestamp` - The timestamp for the event
///
/// # Returns
///
/// A `Writer` containing:
/// - The result: `Either<DomainError, MoneyWithdrawn>`
/// - Accumulated audit entries
pub fn withdraw_with_audit(
    command: &WithdrawCommand,
    account: &Account,
    funding_priority: &[FundingSourceType],
    timestamp: Timestamp,
) -> AuditedResult<MoneyWithdrawn> {
    let initiated_log = withdraw_initiated(
        command.transaction_id,
        command.account_id,
        command.amount.clone(),
    );

    Writer::tell(vec![initiated_log]).flat_map(|()| {
        let validation_result = validate_withdraw_with_priority(command, account, funding_priority);

        match validation_result {
            Either::Right(validated) => {
                let validated_log = withdraw_validated();
                let funding_log = funding_source_selected();
                let event = create_withdraw_event(validated, timestamp);
                let processed_log = withdraw_processed(
                    event.transaction_id,
                    event.account_id,
                    event.amount.clone(),
                );

                Writer::new(
                    Either::Right(event),
                    vec![validated_log, funding_log, processed_log],
                )
            }
            Either::Left(error) => {
                let error_log = validation_failed(format!("{error:?}"));
                Writer::new(Either::Left(error), vec![error_log])
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::aggregate::AccountStatus;
    use crate::domain::account::commands::DepositCommand;
    use crate::domain::audit::AuditAction;
    use crate::domain::value_objects::{AccountId, Currency, Money, TransactionId};
    use rstest::rstest;

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

    fn create_withdraw_command(account_id: AccountId, amount: Money) -> WithdrawCommand {
        WithdrawCommand::new(account_id, amount, TransactionId::generate())
    }

    // =========================================================================
    // deposit_with_audit Tests
    // =========================================================================

    #[rstest]
    fn deposit_with_audit_success_produces_correct_logs() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_right());
        assert_eq!(logs.len(), 3);
        assert!(matches!(logs[0].action, AuditAction::DepositInitiated));
        assert!(matches!(logs[1].action, AuditAction::DepositValidated));
        assert!(matches!(logs[2].action, AuditAction::DepositProcessed));
    }

    #[rstest]
    fn deposit_with_audit_failure_produces_correct_logs() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(-1000, Currency::JPY));
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_left());
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].action, AuditAction::DepositInitiated));
        assert!(matches!(logs[1].action, AuditAction::ValidationFailed));
    }

    #[rstest]
    fn deposit_with_audit_closed_account_produces_correct_logs() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_left());
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].action, AuditAction::DepositInitiated));
        assert!(matches!(logs[1].action, AuditAction::ValidationFailed));
    }

    #[rstest]
    fn deposit_with_audit_logs_contain_correct_context() {
        let account = create_active_account();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let command = DepositCommand::new(account.id, amount, transaction_id);
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (_, logs) = writer.run();

        let initiated_log = &logs[0];
        assert!(initiated_log.context.is_some());
    }

    #[rstest]
    fn deposit_with_audit_returns_correct_event() {
        let account = create_active_account();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let command = DepositCommand::new(account.id, amount.clone(), transaction_id);
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (result, _) = writer.run();

        let event = result.unwrap_right();
        assert_eq!(event.transaction_id, transaction_id);
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, amount);
        assert_eq!(event.balance_after, Money::new(15000, Currency::JPY));
    }

    // =========================================================================
    // withdraw_with_audit Tests
    // =========================================================================

    #[rstest]
    fn withdraw_with_audit_success_produces_correct_logs() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_right());
        assert_eq!(logs.len(), 4);
        assert!(matches!(logs[0].action, AuditAction::WithdrawInitiated));
        assert!(matches!(logs[1].action, AuditAction::WithdrawValidated));
        assert!(matches!(logs[2].action, AuditAction::FundingSourceSelected));
        assert!(matches!(logs[3].action, AuditAction::WithdrawProcessed));
    }

    #[rstest]
    fn withdraw_with_audit_invalid_amount_produces_correct_logs() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(-1000, Currency::JPY));
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_left());
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].action, AuditAction::WithdrawInitiated));
        assert!(matches!(logs[1].action, AuditAction::ValidationFailed));
    }

    #[rstest]
    fn withdraw_with_audit_insufficient_funds_produces_correct_logs() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(50000, Currency::JPY));
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (result, logs) = writer.run();

        // validate_withdraw_with_priority combines validation and funding source
        // selection, so insufficient funds fails during validation
        assert!(result.is_left());
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].action, AuditAction::WithdrawInitiated));
        assert!(matches!(logs[1].action, AuditAction::ValidationFailed));
    }

    #[rstest]
    fn withdraw_with_audit_frozen_account_produces_correct_logs() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (result, logs) = writer.run();

        assert!(result.is_left());
        assert_eq!(logs.len(), 2);
        assert!(matches!(logs[0].action, AuditAction::WithdrawInitiated));
        assert!(matches!(logs[1].action, AuditAction::ValidationFailed));
    }

    #[rstest]
    fn withdraw_with_audit_returns_correct_event() {
        let account = create_active_account();
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let command = WithdrawCommand::new(account.id, amount.clone(), transaction_id);
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (result, _) = writer.run();

        let event = result.unwrap_right();
        assert_eq!(event.transaction_id, transaction_id);
        assert_eq!(event.account_id, account.id);
        assert_eq!(event.amount, amount);
        assert_eq!(event.balance_after, Money::new(5000, Currency::JPY));
    }

    // =========================================================================
    // Writer Monad Law Tests
    // =========================================================================

    #[rstest]
    fn deposit_audit_logs_are_accumulated_in_order() {
        let account = create_active_account();
        let command = create_deposit_command(account.id, Money::new(5000, Currency::JPY));
        let timestamp = Timestamp::now();

        let writer = deposit_with_audit(&command, &account, timestamp);
        let (_, logs) = writer.run();

        assert!(logs[0].timestamp <= logs[1].timestamp);
        assert!(logs[1].timestamp <= logs[2].timestamp);
    }

    #[rstest]
    fn withdraw_audit_logs_are_accumulated_in_order() {
        let account = create_active_account();
        let command = create_withdraw_command(account.id, Money::new(5000, Currency::JPY));
        let funding_priority = [FundingSourceType::Balance];
        let timestamp = Timestamp::now();

        let writer = withdraw_with_audit(&command, &account, &funding_priority, timestamp);
        let (_, logs) = writer.run();

        for window in logs.windows(2) {
            assert!(window[0].timestamp <= window[1].timestamp);
        }
    }
}
