//! Event replay service using Trampoline for stack safety.
//!
//! This module provides stack-safe event replay functionality using
//! the `Trampoline` type from lambars. This allows replaying arbitrarily
//! long event streams without risking stack overflow.
//!
//! # Motivation
//!
//! In event sourcing, replaying events to reconstruct state can involve
//! deeply recursive operations. Rust does not guarantee tail call optimization,
//! so replaying thousands of events could overflow the stack.
//!
//! Using `Trampoline`, we convert the recursive replay into an iterative
//! process that is safe for any number of events.
//!
//! # Examples
//!
//! ```rust
//! use bank::application::services::event_replay::replay_events_safe;
//! use bank::domain::account::aggregate::Account;
//! use bank::domain::account::events::AccountEvent;
//! use lambars::persistent::PersistentList;
//!
//! // Create some events
//! let events: PersistentList<AccountEvent> = PersistentList::new();
//!
//! // Replay events using Trampoline
//! let result = replay_events_safe(
//!     None,
//!     events,
//!     |state, event| Account::apply_event(state, event),
//! );
//!
//! // Execute the trampoline
//! let account = result.run();
//! ```

use lambars::control::Trampoline;
use lambars::persistent::PersistentList;

/// Replays events in a stack-safe manner using Trampoline.
///
/// This function processes a list of events, applying each event to the
/// current state to produce the next state. The recursion is trampolined
/// to avoid stack overflow with large event streams.
///
/// # Type Parameters
///
/// * `S` - The state type (e.g., `Option<Account>`)
/// * `E` - The event type (e.g., `AccountEvent`)
/// * `F` - The apply function type
///
/// # Arguments
///
/// * `initial` - The initial state before any events are applied
/// * `events` - The list of events to replay in order
/// * `apply` - A function that applies an event to a state, producing a new state
///
/// # Returns
///
/// A `Trampoline` that, when run, produces the final state after all events.
///
/// # Design
///
/// The function uses `Trampoline::suspend` to defer each recursive step,
/// converting what would be a stack-consuming recursion into a heap-allocated
/// chain of closures that are executed iteratively.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use bank::application::services::event_replay::replay_events_safe;
/// use bank::domain::account::aggregate::Account;
/// use bank::domain::account::events::{AccountEvent, AccountOpened, EventId};
/// use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
/// use lambars::persistent::PersistentList;
///
/// // Create an AccountOpened event
/// let opened = AccountOpened {
///     event_id: EventId::generate(),
///     account_id: AccountId::generate(),
///     owner_name: "Alice".to_string(),
///     initial_balance: Money::new(10000, Currency::JPY),
///     opened_at: Timestamp::now(),
/// };
///
/// let events = PersistentList::singleton(AccountEvent::Opened(opened));
///
/// // Replay events
/// let result = replay_events_safe(
///     None,
///     events,
///     |state, event| Account::apply_event(state, event),
/// );
///
/// let account = result.run();
/// assert!(account.is_some());
/// ```
///
/// ## With Multiple Events
///
/// ```rust
/// use bank::application::services::event_replay::replay_events_safe;
/// use lambars::persistent::PersistentList;
///
/// // Simple counter example
/// let events = PersistentList::new()
///     .cons(1)
///     .cons(2)
///     .cons(3);
///
/// let result = replay_events_safe(0, events, |sum, &n| sum + n);
/// assert_eq!(result.run(), 6);
/// ```
pub fn replay_events_safe<S, E, F>(initial: S, events: PersistentList<E>, apply: F) -> Trampoline<S>
where
    S: Clone + 'static,
    E: Clone + 'static,
    F: Fn(S, &E) -> S + Clone + 'static,
{
    replay_helper(initial, events.into_iter().collect::<Vec<_>>(), 0, apply)
}

/// Helper function for trampolined event replay.
///
/// Uses an index into a vector to avoid iterator ownership issues with closures.
fn replay_helper<S, E, F>(state: S, events: Vec<E>, index: usize, apply: F) -> Trampoline<S>
where
    S: Clone + 'static,
    E: Clone + 'static,
    F: Fn(S, &E) -> S + Clone + 'static,
{
    if index >= events.len() {
        Trampoline::done(state)
    } else {
        let new_state = apply(state, &events[index]);
        Trampoline::suspend(move || replay_helper(new_state, events, index + 1, apply))
    }
}

/// Replays events with early termination support.
///
/// Similar to `replay_events_safe`, but the apply function returns `Option<S>`.
/// If the apply function returns `None`, replay stops and returns `None`.
///
/// # Type Parameters
///
/// * `S` - The state type
/// * `E` - The event type
/// * `F` - The apply function type
///
/// # Arguments
///
/// * `initial` - The initial state (can be `None` for fresh aggregates)
/// * `events` - The list of events to replay
/// * `apply` - A function that applies an event, returning `Some(state)` or `None`
///
/// # Returns
///
/// A `Trampoline` that produces `Some(state)` if all events applied successfully,
/// or `None` if any event application failed.
///
/// # Examples
///
/// ```rust
/// use bank::application::services::event_replay::replay_events_safe_with_option;
/// use lambars::persistent::PersistentList;
///
/// // Only accept positive numbers
/// let events = PersistentList::new().cons(3).cons(2).cons(1);
///
/// let result = replay_events_safe_with_option(
///     Some(0),
///     events,
///     |state, &n| state.map(|s| s + n).filter(|&s| s <= 10),
/// );
///
/// assert_eq!(result.run(), Some(6));
/// ```
pub fn replay_events_safe_with_option<S, E, F>(
    initial: Option<S>,
    events: PersistentList<E>,
    apply: F,
) -> Trampoline<Option<S>>
where
    S: Clone + 'static,
    E: Clone + 'static,
    F: Fn(Option<S>, &E) -> Option<S> + Clone + 'static,
{
    replay_option_helper(initial, events.into_iter().collect::<Vec<_>>(), 0, apply)
}

/// Helper function for trampolined event replay with Option.
fn replay_option_helper<S, E, F>(
    state: Option<S>,
    events: Vec<E>,
    index: usize,
    apply: F,
) -> Trampoline<Option<S>>
where
    S: Clone + 'static,
    E: Clone + 'static,
    F: Fn(Option<S>, &E) -> Option<S> + Clone + 'static,
{
    if index >= events.len() {
        Trampoline::done(state)
    } else {
        match state {
            None if index > 0 => Trampoline::done(None), // Early termination
            _ => {
                let new_state = apply(state, &events[index]);
                Trampoline::suspend(move || {
                    replay_option_helper(new_state, events, index + 1, apply)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::aggregate::Account;
    use crate::domain::account::events::{AccountEvent, AccountOpened, EventId, MoneyDeposited};
    use crate::domain::value_objects::{AccountId, Currency, Money, Timestamp, TransactionId};
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_account_opened_event(account_id: AccountId, initial_balance: Money) -> AccountEvent {
        AccountEvent::Opened(AccountOpened {
            event_id: EventId::generate(),
            account_id,
            owner_name: "Alice".to_string(),
            initial_balance,
            opened_at: Timestamp::now(),
        })
    }

    fn create_deposit_event(
        account_id: AccountId,
        amount: Money,
        balance_after: Money,
    ) -> AccountEvent {
        AccountEvent::Deposited(MoneyDeposited {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            amount,
            balance_after,
            deposited_at: Timestamp::now(),
        })
    }

    // =========================================================================
    // replay_events_safe Tests
    // =========================================================================

    #[rstest]
    fn replay_events_safe_empty_list_returns_initial() {
        let events: PersistentList<i32> = PersistentList::new();

        let result = replay_events_safe(0, events, |sum, &n| sum + n);

        assert_eq!(result.run(), 0);
    }

    #[rstest]
    fn replay_events_safe_single_event() {
        let events = PersistentList::singleton(5);

        let result = replay_events_safe(0, events, |sum, &n| sum + n);

        assert_eq!(result.run(), 5);
    }

    #[rstest]
    fn replay_events_safe_multiple_events() {
        let events = PersistentList::new().cons(3).cons(2).cons(1);

        let result = replay_events_safe(0, events, |sum, &n| sum + n);

        assert_eq!(result.run(), 6);
    }

    #[rstest]
    fn replay_events_safe_with_account_events() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);

        let events = PersistentList::singleton(create_account_opened_event(
            account_id,
            initial_balance.clone(),
        ));

        let result = replay_events_safe(None, events, |state, event| {
            Account::apply_event(state, event)
        });

        let account = result.run();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.id, account_id);
        assert_eq!(account.balance, initial_balance);
    }

    #[rstest]
    fn replay_events_safe_with_multiple_account_events() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let deposit_amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(15000, Currency::JPY);

        let events = PersistentList::new()
            .cons(create_deposit_event(
                account_id,
                deposit_amount,
                balance_after.clone(),
            ))
            .cons(create_account_opened_event(account_id, initial_balance));

        let result = replay_events_safe(None, events, |state, event| {
            Account::apply_event(state, event)
        });

        let account = result.run();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn replay_events_safe_handles_many_events() {
        // Test with a moderately large number of events to verify stack safety
        let mut events = PersistentList::new();
        for i in 0..1000 {
            events = events.cons(i);
        }

        let result = replay_events_safe(0i64, events, |sum, &n| sum + n);
        let total = result.run();

        // Sum of 0 to 999
        assert_eq!(total, (0..1000i64).sum::<i64>());
    }

    #[rstest]
    fn replay_events_safe_preserves_order() {
        let events = PersistentList::new().cons(3).cons(2).cons(1);

        let result = replay_events_safe(Vec::new(), events, |mut vec, &n| {
            vec.push(n);
            vec
        });

        assert_eq!(result.run(), vec![1, 2, 3]);
    }

    // =========================================================================
    // replay_events_safe_with_option Tests
    // =========================================================================

    #[rstest]
    fn replay_events_safe_with_option_empty_list() {
        let events: PersistentList<i32> = PersistentList::new();

        let result =
            replay_events_safe_with_option(Some(0), events, |state, &n| state.map(|s| s + n));

        assert_eq!(result.run(), Some(0));
    }

    #[rstest]
    fn replay_events_safe_with_option_all_succeed() {
        let events = PersistentList::new().cons(3).cons(2).cons(1);

        let result =
            replay_events_safe_with_option(Some(0), events, |state, &n| state.map(|s| s + n));

        assert_eq!(result.run(), Some(6));
    }

    #[rstest]
    fn replay_events_safe_with_option_with_account_events() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);

        let events = PersistentList::singleton(create_account_opened_event(
            account_id,
            initial_balance.clone(),
        ));

        let result = replay_events_safe_with_option(None, events, |state, event| {
            Account::apply_event(state, event)
        });

        let account = result.run();
        assert!(account.is_some());
        assert_eq!(account.unwrap().balance, initial_balance);
    }

    #[rstest]
    fn replay_events_safe_with_option_initial_none_with_valid_first_event() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);

        let events =
            PersistentList::singleton(create_account_opened_event(account_id, initial_balance));

        let result = replay_events_safe_with_option(None, events, |state, event| {
            Account::apply_event(state, event)
        });

        assert!(result.run().is_some());
    }

    // =========================================================================
    // Trampoline Integration Tests
    // =========================================================================

    #[rstest]
    fn trampoline_is_stack_safe() {
        // Create a very deep event list
        let mut events = PersistentList::new();
        for i in 0..10000 {
            events = events.cons(i);
        }

        let result = replay_events_safe(0i64, events, |sum, &n| sum + n);

        // This should not stack overflow
        let total = result.run();
        assert_eq!(total, (0..10000i64).sum::<i64>());
    }

    // =========================================================================
    // Referential Transparency Tests
    // =========================================================================

    #[rstest]
    fn replay_is_referentially_transparent() {
        let events = PersistentList::new().cons(3).cons(2).cons(1);

        let result1 = replay_events_safe(0, events.clone(), |sum, &n| sum + n);
        let result2 = replay_events_safe(0, events, |sum, &n| sum + n);

        assert_eq!(result1.run(), result2.run());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[rstest]
    fn replay_with_string_events() {
        let events = PersistentList::new()
            .cons("world".to_string())
            .cons(" ".to_string())
            .cons("Hello".to_string());

        let result = replay_events_safe(String::new(), events, |mut s, e| {
            s.push_str(e);
            s
        });

        assert_eq!(result.run(), "Hello world");
    }

    #[rstest]
    fn replay_with_complex_state() {
        #[derive(Clone, Debug, PartialEq)]
        struct State {
            count: i32,
            sum: i32,
        }

        let events = PersistentList::new().cons(3).cons(2).cons(1);

        let result = replay_events_safe(State { count: 0, sum: 0 }, events, |mut s, &n| {
            s.count += 1;
            s.sum += n;
            s
        });

        let final_state = result.run();
        assert_eq!(final_state.count, 3);
        assert_eq!(final_state.sum, 6);
    }
}
