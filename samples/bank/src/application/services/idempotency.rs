//! Idempotency checking service for transaction operations.
//!
//! This module provides pure functions to check if a transaction has
//! already been processed by examining existing events.
//!
//! # Design Principles
//!
//! - **Pure Functions**: All checking functions are referentially transparent
//! - **Event Sourcing**: Uses the event stream as the source of truth
//! - **No Side Effects**: Does not modify any state
//!
//! # Usage
//!
//! ```rust,ignore
//! use bank::application::services::idempotency::{
//!     check_transaction_idempotency, IdempotencyCheckResult,
//! };
//!
//! let result = check_transaction_idempotency(&events, &transaction_id);
//! match result {
//!     IdempotencyCheckResult::NotFound => { /* proceed with new transaction */ }
//!     IdempotencyCheckResult::AlreadyProcessed(event) => { /* return existing result */ }
//! }
//! ```

use crate::domain::account::events::AccountEvent;
use crate::domain::value_objects::TransactionId;
use lambars::persistent::PersistentList;

/// Result of an idempotency check.
///
/// This type represents whether a transaction with a given ID has already
/// been processed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyCheckResult {
    /// No existing event found with the given transaction ID.
    /// It is safe to proceed with processing the new transaction.
    NotFound,

    /// Found an existing event with the same transaction ID.
    /// The event is returned so that the handler can return the
    /// same response without reprocessing.
    AlreadyProcessed(AccountEvent),
}

/// Checks if a transaction with the given ID has already been processed.
///
/// This is a pure function that examines the event stream to find
/// any event with a matching transaction ID.
///
/// # Arguments
///
/// * `events` - The list of events for an aggregate
/// * `transaction_id` - The transaction ID to check
///
/// # Returns
///
/// * [`IdempotencyCheckResult::NotFound`] if no matching event exists
/// * [`IdempotencyCheckResult::AlreadyProcessed`] if a matching event exists
///
/// # Complexity
///
/// O(n) where n is the number of events in the list.
///
/// # Example
///
/// ```rust,ignore
/// let events = event_store.load_events(&account_id).run_async().await?;
/// let transaction_id = TransactionId::from_idempotency_key("user-123-deposit-001");
///
/// match check_transaction_idempotency(&events, &transaction_id) {
///     IdempotencyCheckResult::NotFound => {
///         // Proceed with new transaction
///     }
///     IdempotencyCheckResult::AlreadyProcessed(existing_event) => {
///         // Return existing result
///     }
/// }
/// ```
#[must_use]
pub fn check_transaction_idempotency(
    events: &PersistentList<AccountEvent>,
    transaction_id: &TransactionId,
) -> IdempotencyCheckResult {
    for event in events {
        if let Some(existing_id) = event.transaction_id()
            && existing_id == transaction_id
        {
            return IdempotencyCheckResult::AlreadyProcessed(event.clone());
        }
    }
    IdempotencyCheckResult::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::events::{EventId, MoneyDeposited, MoneyWithdrawn};
    use crate::domain::value_objects::{AccountId, Currency, Money, Timestamp};
    use rstest::rstest;

    fn create_deposit_event(transaction_id: TransactionId) -> AccountEvent {
        AccountEvent::Deposited(MoneyDeposited {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id,
            amount: Money::new(1000, Currency::JPY),
            balance_after: Money::new(11000, Currency::JPY),
            deposited_at: Timestamp::now(),
        })
    }

    fn create_withdrawal_event(transaction_id: TransactionId) -> AccountEvent {
        AccountEvent::Withdrawn(MoneyWithdrawn {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id,
            amount: Money::new(500, Currency::JPY),
            balance_after: Money::new(10500, Currency::JPY),
            withdrawn_at: Timestamp::now(),
        })
    }

    #[rstest]
    fn check_idempotency_empty_events_returns_not_found() {
        let events = PersistentList::new();
        let transaction_id = TransactionId::from_idempotency_key("test-key-1");

        let result = check_transaction_idempotency(&events, &transaction_id);

        assert_eq!(result, IdempotencyCheckResult::NotFound);
    }

    #[rstest]
    fn check_idempotency_no_matching_transaction_returns_not_found() {
        let transaction_id_1 = TransactionId::from_idempotency_key("test-key-1");
        let transaction_id_2 = TransactionId::from_idempotency_key("test-key-2");

        let events = PersistentList::new().cons(create_deposit_event(transaction_id_1));

        let result = check_transaction_idempotency(&events, &transaction_id_2);

        assert_eq!(result, IdempotencyCheckResult::NotFound);
    }

    #[rstest]
    fn check_idempotency_matching_deposit_returns_already_processed() {
        let transaction_id = TransactionId::from_idempotency_key("test-key-1");
        let event = create_deposit_event(transaction_id);

        let events = PersistentList::new().cons(event.clone());

        let result = check_transaction_idempotency(&events, &transaction_id);

        assert_eq!(result, IdempotencyCheckResult::AlreadyProcessed(event));
    }

    #[rstest]
    fn check_idempotency_matching_withdrawal_returns_already_processed() {
        let transaction_id = TransactionId::from_idempotency_key("test-key-1");
        let event = create_withdrawal_event(transaction_id);

        let events = PersistentList::new().cons(event.clone());

        let result = check_transaction_idempotency(&events, &transaction_id);

        assert_eq!(result, IdempotencyCheckResult::AlreadyProcessed(event));
    }

    #[rstest]
    fn check_idempotency_finds_match_in_multiple_events() {
        let transaction_id_1 = TransactionId::from_idempotency_key("test-key-1");
        let transaction_id_2 = TransactionId::from_idempotency_key("test-key-2");
        let transaction_id_3 = TransactionId::from_idempotency_key("test-key-3");

        let event_2 = create_deposit_event(transaction_id_2);

        let events = PersistentList::new()
            .cons(create_deposit_event(transaction_id_1))
            .cons(event_2.clone())
            .cons(create_withdrawal_event(transaction_id_3));

        let result = check_transaction_idempotency(&events, &transaction_id_2);

        assert_eq!(result, IdempotencyCheckResult::AlreadyProcessed(event_2));
    }

    #[rstest]
    fn check_idempotency_is_referentially_transparent() {
        let transaction_id = TransactionId::from_idempotency_key("test-key-1");
        let event = create_deposit_event(transaction_id);
        let events = PersistentList::new().cons(event);

        let result1 = check_transaction_idempotency(&events, &transaction_id);
        let result2 = check_transaction_idempotency(&events, &transaction_id);

        assert_eq!(result1, result2);
    }
}
