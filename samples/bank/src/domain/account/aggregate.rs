//! Account aggregate for the bank domain.
//!
//! This module defines the `Account` aggregate root, which represents a bank account
//! and its associated business rules. The aggregate uses functional programming patterns:
//!
//! - **Lens**: For immutable updates to account fields
//! - **Foldable**: For event replay (reconstructing state from events)
//! - **Either**: For error handling in domain operations
//!
//! # Design Principles
//!
//! - **Immutability**: All operations return new `Account` instances
//! - **Pure Functions**: Domain logic is side-effect free
//! - **Event Sourcing**: State is derived from events via `from_events`
//!
//! # Examples
//!
//! ```rust
//! use bank::domain::account::aggregate::{Account, AccountStatus};
//! use bank::domain::account::events::{AccountEvent, AccountOpened, EventId};
//! use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
//! use lambars::persistent::PersistentList;
//! use lambars::optics::Lens;
//!
//! // Create events
//! let opened = AccountOpened {
//!     event_id: EventId::generate(),
//!     account_id: AccountId::generate(),
//!     owner_name: "Alice".to_string(),
//!     initial_balance: Money::new(10000, Currency::JPY),
//!     opened_at: Timestamp::now(),
//! };
//!
//! // Replay events to reconstruct state
//! let events = PersistentList::new().cons(AccountEvent::Opened(opened));
//! let account = Account::from_events(&events);
//! assert!(account.is_some());
//! ```

use lambars::control::Either;
use lambars::optics::{FunctionLens, Lens};
use lambars::persistent::PersistentList;
use lambars::typeclass::Foldable;
use serde::{Deserialize, Serialize};

use crate::domain::account::errors::{DomainError, DomainResult};
use crate::domain::account::events::{
    AccountClosed, AccountEvent, AccountOpened, MoneyDeposited, MoneyWithdrawn, TransferReceived,
    TransferSent,
};
use crate::domain::value_objects::{AccountId, Money};

/// The status of a bank account.
///
/// Represents the current operational state of an account, which determines
/// what operations are permitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    /// The account is active and can perform all operations.
    Active,
    /// The account is frozen and operations are temporarily suspended.
    Frozen,
    /// The account is closed and cannot perform any operations.
    Closed,
}

impl AccountStatus {
    /// Returns `true` if the account status is `Active`.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the account status is `Frozen`.
    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        matches!(self, Self::Frozen)
    }

    /// Returns `true` if the account status is `Closed`.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }
}

/// A bank account aggregate.
///
/// `Account` is the aggregate root for bank account operations. It contains
/// all the state needed to make business decisions and enforces invariants.
///
/// # Immutability
///
/// `Account` is immutable. All modifications create new instances:
///
/// ```rust
/// use bank::domain::account::aggregate::Account;
/// use lambars::optics::Lens;
///
/// // Use Lens to create a modified copy
/// // let new_account = Account::balance_lens().set(account, new_balance);
/// ```
///
/// # Event Sourcing
///
/// Account state can be reconstructed from events:
///
/// ```rust
/// use bank::domain::account::aggregate::Account;
/// use bank::domain::account::events::AccountEvent;
/// use lambars::persistent::PersistentList;
///
/// // let account = Account::from_events(&event_list);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The unique identifier for this account.
    pub id: AccountId,
    /// The name of the account owner.
    pub owner_name: String,
    /// The current balance of the account.
    pub balance: Money,
    /// The current status of the account.
    pub status: AccountStatus,
    /// The version number for optimistic concurrency control.
    pub version: u64,
}

impl Account {
    // =========================================================================
    // Lens Accessors
    // =========================================================================

    /// Creates a Lens for the `balance` field.
    ///
    /// This lens enables immutable updates to the account balance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::Account;
    /// use bank::domain::value_objects::{Money, Currency};
    /// use lambars::optics::Lens;
    ///
    /// // let updated = Account::balance_lens().set(account, Money::new(5000, Currency::JPY));
    /// ```
    #[must_use]
    pub fn balance_lens() -> impl Lens<Self, Money> + Clone {
        FunctionLens::new(
            |account: &Self| &account.balance,
            |mut account: Self, balance: Money| {
                account.balance = balance;
                account
            },
        )
    }

    /// Creates a Lens for the `status` field.
    ///
    /// This lens enables immutable updates to the account status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::{Account, AccountStatus};
    /// use lambars::optics::Lens;
    ///
    /// // let closed = Account::status_lens().set(account, AccountStatus::Closed);
    /// ```
    #[must_use]
    pub fn status_lens() -> impl Lens<Self, AccountStatus> + Clone {
        FunctionLens::new(
            |account: &Self| &account.status,
            |mut account: Self, status: AccountStatus| {
                account.status = status;
                account
            },
        )
    }

    /// Creates a Lens for the `version` field.
    ///
    /// This lens enables immutable updates to the version number.
    #[must_use]
    pub fn version_lens() -> impl Lens<Self, u64> + Clone {
        FunctionLens::new(
            |account: &Self| &account.version,
            |mut account: Self, version: u64| {
                account.version = version;
                account
            },
        )
    }

    // =========================================================================
    // Event Sourcing (Foldable)
    // =========================================================================

    /// Reconstructs an account from a list of events.
    ///
    /// Uses `Foldable::fold_left` to apply events in order, starting from `None`.
    /// Returns `None` if no `AccountOpened` event is found or events are invalid.
    ///
    /// # Arguments
    ///
    /// * `events` - A persistent list of account events in chronological order
    ///
    /// # Returns
    ///
    /// * `Some(Account)` if the events form a valid account history
    /// * `None` if the events are empty or don't start with `AccountOpened`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::Account;
    /// use bank::domain::account::events::{AccountEvent, AccountOpened, EventId};
    /// use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp};
    /// use lambars::persistent::PersistentList;
    ///
    /// let opened = AccountOpened {
    ///     event_id: EventId::generate(),
    ///     account_id: AccountId::generate(),
    ///     owner_name: "Alice".to_string(),
    ///     initial_balance: Money::new(10000, Currency::JPY),
    ///     opened_at: Timestamp::now(),
    /// };
    ///
    /// let events = PersistentList::singleton(AccountEvent::Opened(opened));
    /// let account = Account::from_events(&events);
    /// assert!(account.is_some());
    /// ```
    #[must_use]
    pub fn from_events(events: &PersistentList<AccountEvent>) -> Option<Self> {
        events
            .clone()
            .fold_left(None, |state: Option<Self>, event: AccountEvent| {
                Self::apply_event(state, &event)
            })
    }

    /// Applies a single event to an optional account state.
    ///
    /// This is a pure function that returns a new state based on the event.
    /// It handles the initial `AccountOpened` event specially, creating the
    /// initial account state.
    ///
    /// # Arguments
    ///
    /// * `state` - The current account state (None if account doesn't exist yet)
    /// * `event` - The event to apply
    ///
    /// # Returns
    ///
    /// * `Some(Account)` with the updated state
    /// * `None` if the event cannot be applied to the current state
    ///
    /// # State Transitions
    ///
    /// | Current State | Event | Result |
    /// |--------------|-------|--------|
    /// | None | AccountOpened | Some(new account) |
    /// | None | Other events | None |
    /// | Some(account) | MoneyDeposited | Some(updated balance) |
    /// | Some(account) | MoneyWithdrawn | Some(updated balance) |
    /// | Some(account) | TransferSent | Some(updated balance) |
    /// | Some(account) | TransferReceived | Some(updated balance) |
    /// | Some(account) | AccountClosed | Some(closed status) |
    /// | Some(account) | AccountOpened | None (invalid) |
    #[must_use]
    pub fn apply_event(state: Option<Self>, event: &AccountEvent) -> Option<Self> {
        match (state, event) {
            // Initial state: Create account from AccountOpened
            (
                None,
                AccountEvent::Opened(AccountOpened {
                    account_id,
                    owner_name,
                    initial_balance,
                    ..
                }),
            ) => Some(Self {
                id: *account_id,
                owner_name: owner_name.clone(),
                balance: initial_balance.clone(),
                status: AccountStatus::Active,
                version: 1,
            }),

            // Update balance on deposit, withdrawal, transfer sent, or transfer received
            (
                Some(account),
                AccountEvent::Deposited(MoneyDeposited { balance_after, .. })
                | AccountEvent::Withdrawn(MoneyWithdrawn { balance_after, .. })
                | AccountEvent::TransferSent(TransferSent { balance_after, .. })
                | AccountEvent::TransferReceived(TransferReceived { balance_after, .. }),
            ) => Some(Self {
                balance: balance_after.clone(),
                version: account.version + 1,
                ..account
            }),

            // Close account
            (Some(account), AccountEvent::Closed(AccountClosed { .. })) => Some(Self {
                status: AccountStatus::Closed,
                version: account.version + 1,
                ..account
            }),

            // Invalid transitions: Can't apply events without an account or open an existing account
            (None, _) | (Some(_), AccountEvent::Opened(_)) => None,
        }
    }

    // =========================================================================
    // Domain Logic (Pure Functions)
    // =========================================================================

    /// Checks if the account can withdraw the specified amount.
    ///
    /// This is a pure function that returns a `DomainResult` indicating
    /// whether the withdrawal is permitted.
    ///
    /// # Arguments
    ///
    /// * `amount` - The amount to withdraw
    ///
    /// # Returns
    ///
    /// * `Either::Right(())` if withdrawal is permitted
    /// * `Either::Left(DomainError)` if withdrawal is not permitted
    ///
    /// # Validation Rules
    ///
    /// 1. Account must be active (not closed or frozen)
    /// 2. Account must have sufficient balance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::{Account, AccountStatus};
    /// use bank::domain::value_objects::{AccountId, Money, Currency};
    ///
    /// // Assuming account has balance of 10000 JPY
    /// // let result = account.can_withdraw(&Money::new(5000, Currency::JPY));
    /// // assert!(result.is_right());
    /// ```
    pub fn can_withdraw(&self, amount: &Money) -> DomainResult<()> {
        // Check account status
        match self.status {
            AccountStatus::Closed => {
                return Either::Left(DomainError::AccountClosed(self.id));
            }
            AccountStatus::Frozen => {
                return Either::Left(DomainError::AccountFrozen(self.id));
            }
            AccountStatus::Active => {}
        }

        // Check balance
        if self.balance < *amount {
            return Either::Left(DomainError::InsufficientBalance {
                required: amount.clone(),
                available: self.balance.clone(),
            });
        }

        Either::Right(())
    }

    /// Checks if the account can accept a deposit.
    ///
    /// This is a pure function that returns a `DomainResult` indicating
    /// whether deposits are permitted.
    ///
    /// # Returns
    ///
    /// * `Either::Right(())` if deposits are permitted
    /// * `Either::Left(DomainError)` if deposits are not permitted
    ///
    /// # Validation Rules
    ///
    /// 1. Account must not be closed
    /// 2. Frozen accounts can still receive deposits
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::Account;
    ///
    /// // let result = account.can_deposit();
    /// // assert!(result.is_right());
    /// ```
    pub const fn can_deposit(&self) -> DomainResult<()> {
        match self.status {
            AccountStatus::Closed => Either::Left(DomainError::AccountClosed(self.id)),
            AccountStatus::Active | AccountStatus::Frozen => Either::Right(()),
        }
    }

    /// Returns `true` if the account is in active status.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::domain::account::aggregate::{Account, AccountStatus};
    ///
    /// // let is_active = account.is_active();
    /// ```
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns `true` if the account is frozen.
    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        self.status.is_frozen()
    }

    /// Returns `true` if the account is closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.status.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::events::EventId;
    use crate::domain::value_objects::{Currency, Timestamp, TransactionId};
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_account_opened_event(
        account_id: AccountId,
        owner_name: &str,
        initial_balance: Money,
    ) -> AccountEvent {
        AccountEvent::Opened(AccountOpened {
            event_id: EventId::generate(),
            account_id,
            owner_name: owner_name.to_string(),
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

    fn create_withdrawal_event(
        account_id: AccountId,
        amount: Money,
        balance_after: Money,
    ) -> AccountEvent {
        AccountEvent::Withdrawn(MoneyWithdrawn {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            amount,
            balance_after,
            withdrawn_at: Timestamp::now(),
        })
    }

    fn create_transfer_sent_event(
        account_id: AccountId,
        to_account_id: AccountId,
        amount: Money,
        balance_after: Money,
    ) -> AccountEvent {
        AccountEvent::TransferSent(TransferSent {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            to_account_id,
            amount,
            balance_after,
            sent_at: Timestamp::now(),
        })
    }

    fn create_transfer_received_event(
        account_id: AccountId,
        from_account_id: AccountId,
        amount: Money,
        balance_after: Money,
    ) -> AccountEvent {
        AccountEvent::TransferReceived(TransferReceived {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            from_account_id,
            amount,
            balance_after,
            received_at: Timestamp::now(),
        })
    }

    fn create_account_closed_event(account_id: AccountId, final_balance: Money) -> AccountEvent {
        AccountEvent::Closed(AccountClosed {
            event_id: EventId::generate(),
            account_id,
            closed_at: Timestamp::now(),
            final_balance,
        })
    }

    fn create_active_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: Money::new(10000, Currency::JPY),
            status: AccountStatus::Active,
            version: 1,
        }
    }

    // =========================================================================
    // AccountStatus Tests
    // =========================================================================

    #[rstest]
    fn account_status_is_active_returns_true_for_active() {
        assert!(AccountStatus::Active.is_active());
        assert!(!AccountStatus::Frozen.is_active());
        assert!(!AccountStatus::Closed.is_active());
    }

    #[rstest]
    fn account_status_is_frozen_returns_true_for_frozen() {
        assert!(!AccountStatus::Active.is_frozen());
        assert!(AccountStatus::Frozen.is_frozen());
        assert!(!AccountStatus::Closed.is_frozen());
    }

    #[rstest]
    fn account_status_is_closed_returns_true_for_closed() {
        assert!(!AccountStatus::Active.is_closed());
        assert!(!AccountStatus::Frozen.is_closed());
        assert!(AccountStatus::Closed.is_closed());
    }

    // =========================================================================
    // Lens Tests
    // =========================================================================

    #[rstest]
    fn balance_lens_get_returns_balance() {
        let account = create_active_account();
        let lens = Account::balance_lens();

        assert_eq!(*lens.get(&account), account.balance);
    }

    #[rstest]
    fn balance_lens_set_updates_balance() {
        let account = create_active_account();
        let lens = Account::balance_lens();
        let new_balance = Money::new(20000, Currency::JPY);

        let updated = lens.set(account.clone(), new_balance.clone());

        assert_eq!(updated.balance, new_balance);
        assert_eq!(updated.id, account.id);
        assert_eq!(updated.owner_name, account.owner_name);
    }

    #[rstest]
    fn balance_lens_modify_transforms_balance() {
        let account = create_active_account();
        let lens = Account::balance_lens();

        // Note: This test uses set since Money doesn't have simple arithmetic
        let doubled_amount = Money::new(20000, Currency::JPY);
        let updated = lens.set(account, doubled_amount.clone());

        assert_eq!(updated.balance, doubled_amount);
    }

    #[rstest]
    fn status_lens_get_returns_status() {
        let account = create_active_account();
        let lens = Account::status_lens();

        assert_eq!(*lens.get(&account), AccountStatus::Active);
    }

    #[rstest]
    fn status_lens_set_updates_status() {
        let account = create_active_account();
        let lens = Account::status_lens();

        let updated = lens.set(account.clone(), AccountStatus::Closed);

        assert_eq!(updated.status, AccountStatus::Closed);
        assert_eq!(updated.balance, account.balance);
    }

    #[rstest]
    fn version_lens_get_returns_version() {
        let account = create_active_account();
        let lens = Account::version_lens();

        assert_eq!(*lens.get(&account), 1);
    }

    #[rstest]
    fn version_lens_set_updates_version() {
        let account = create_active_account();
        let lens = Account::version_lens();

        let updated = lens.set(account, 5);

        assert_eq!(updated.version, 5);
    }

    // =========================================================================
    // Lens Laws Tests
    // =========================================================================

    #[rstest]
    fn balance_lens_get_put_law() {
        let account = create_active_account();
        let lens = Account::balance_lens();

        // GetPut: lens.set(s, lens.get(&s).clone()) == s
        let result = lens.set(account.clone(), lens.get(&account).clone());
        assert_eq!(result, account);
    }

    #[rstest]
    fn balance_lens_put_get_law() {
        let account = create_active_account();
        let lens = Account::balance_lens();
        let new_balance = Money::new(5000, Currency::JPY);

        // PutGet: lens.get(&lens.set(s, v)) == &v
        let updated = lens.set(account, new_balance.clone());
        assert_eq!(*lens.get(&updated), new_balance);
    }

    #[rstest]
    fn balance_lens_put_put_law() {
        let account = create_active_account();
        let lens = Account::balance_lens();
        let balance1 = Money::new(5000, Currency::JPY);
        let balance2 = Money::new(8000, Currency::JPY);

        // PutPut: lens.set(lens.set(s, v1), v2) == lens.set(s, v2)
        let left = lens.set(lens.set(account.clone(), balance1), balance2.clone());
        let right = lens.set(account, balance2);
        assert_eq!(left, right);
    }

    // =========================================================================
    // from_events Tests
    // =========================================================================

    #[rstest]
    fn from_events_empty_list_returns_none() {
        let events: PersistentList<AccountEvent> = PersistentList::new();
        let account = Account::from_events(&events);

        assert!(account.is_none());
    }

    #[rstest]
    fn from_events_with_opened_returns_account() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let event = create_account_opened_event(account_id, "Alice", initial_balance.clone());

        let events = PersistentList::singleton(event);
        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.id, account_id);
        assert_eq!(account.owner_name, "Alice");
        assert_eq!(account.balance, initial_balance);
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.version, 1);
    }

    #[rstest]
    fn from_events_with_deposit_updates_balance() {
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
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn from_events_with_withdrawal_updates_balance() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let withdrawal_amount = Money::new(3000, Currency::JPY);
        let balance_after = Money::new(7000, Currency::JPY);

        let events = PersistentList::new()
            .cons(create_withdrawal_event(
                account_id,
                withdrawal_amount,
                balance_after.clone(),
            ))
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn from_events_with_transfer_sent_updates_balance() {
        let account_id = AccountId::generate();
        let to_account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let transfer_amount = Money::new(2000, Currency::JPY);
        let balance_after = Money::new(8000, Currency::JPY);

        let events = PersistentList::new()
            .cons(create_transfer_sent_event(
                account_id,
                to_account_id,
                transfer_amount,
                balance_after.clone(),
            ))
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn from_events_with_transfer_received_updates_balance() {
        let account_id = AccountId::generate();
        let from_account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let transfer_amount = Money::new(2000, Currency::JPY);
        let balance_after = Money::new(12000, Currency::JPY);

        let events = PersistentList::new()
            .cons(create_transfer_received_event(
                account_id,
                from_account_id,
                transfer_amount,
                balance_after.clone(),
            ))
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn from_events_with_closed_updates_status() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let final_balance = Money::zero(Currency::JPY);

        let events = PersistentList::new()
            .cons(create_account_closed_event(account_id, final_balance))
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.status, AccountStatus::Closed);
        assert_eq!(account.version, 2);
    }

    #[rstest]
    fn from_events_multiple_events_applies_in_order() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let deposit_amount = Money::new(5000, Currency::JPY);
        let withdrawal_amount = Money::new(3000, Currency::JPY);
        let balance_after_deposit = Money::new(15000, Currency::JPY);
        let balance_after_withdrawal = Money::new(12000, Currency::JPY);

        let events = PersistentList::new()
            .cons(create_withdrawal_event(
                account_id,
                withdrawal_amount,
                balance_after_withdrawal.clone(),
            ))
            .cons(create_deposit_event(
                account_id,
                deposit_amount,
                balance_after_deposit,
            ))
            .cons(create_account_opened_event(
                account_id,
                "Alice",
                initial_balance,
            ));

        let account = Account::from_events(&events);

        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.balance, balance_after_withdrawal);
        assert_eq!(account.version, 3);
    }

    // =========================================================================
    // apply_event Tests
    // =========================================================================

    #[rstest]
    fn apply_event_opened_on_none_creates_account() {
        let account_id = AccountId::generate();
        let initial_balance = Money::new(10000, Currency::JPY);
        let event = create_account_opened_event(account_id, "Alice", initial_balance.clone());

        let result = Account::apply_event(None, &event);

        assert!(result.is_some());
        let account = result.unwrap();
        assert_eq!(account.id, account_id);
        assert_eq!(account.owner_name, "Alice");
        assert_eq!(account.balance, initial_balance);
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.version, 1);
    }

    #[rstest]
    fn apply_event_deposit_on_none_returns_none() {
        let account_id = AccountId::generate();
        let event = create_deposit_event(
            account_id,
            Money::new(5000, Currency::JPY),
            Money::new(15000, Currency::JPY),
        );

        let result = Account::apply_event(None, &event);

        assert!(result.is_none());
    }

    #[rstest]
    fn apply_event_opened_on_some_returns_none() {
        let account = create_active_account();
        let event = create_account_opened_event(
            AccountId::generate(),
            "Bob",
            Money::new(5000, Currency::JPY),
        );

        let result = Account::apply_event(Some(account), &event);

        assert!(result.is_none());
    }

    // =========================================================================
    // can_withdraw Tests
    // =========================================================================

    #[rstest]
    fn can_withdraw_active_with_sufficient_balance_returns_right() {
        let account = create_active_account();
        let amount = Money::new(5000, Currency::JPY);

        let result = account.can_withdraw(&amount);

        assert!(result.is_right());
    }

    #[rstest]
    fn can_withdraw_active_with_exact_balance_returns_right() {
        let account = create_active_account();
        let amount = Money::new(10000, Currency::JPY);

        let result = account.can_withdraw(&amount);

        assert!(result.is_right());
    }

    #[rstest]
    fn can_withdraw_active_with_insufficient_balance_returns_left() {
        let account = create_active_account();
        let amount = Money::new(15000, Currency::JPY);

        let result = account.can_withdraw(&amount);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::InsufficientBalance {
            required,
            available,
        } = error
        {
            assert_eq!(required, amount);
            assert_eq!(available, account.balance);
        } else {
            panic!("Expected InsufficientBalance error");
        }
    }

    #[rstest]
    fn can_withdraw_closed_returns_account_closed() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;
        let amount = Money::new(1000, Currency::JPY);

        let result = account.can_withdraw(&amount);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::AccountClosed(id) = error {
            assert_eq!(id, account.id);
        } else {
            panic!("Expected AccountClosed error");
        }
    }

    #[rstest]
    fn can_withdraw_frozen_returns_account_frozen() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;
        let amount = Money::new(1000, Currency::JPY);

        let result = account.can_withdraw(&amount);

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::AccountFrozen(id) = error {
            assert_eq!(id, account.id);
        } else {
            panic!("Expected AccountFrozen error");
        }
    }

    // =========================================================================
    // can_deposit Tests
    // =========================================================================

    #[rstest]
    fn can_deposit_active_returns_right() {
        let account = create_active_account();

        let result = account.can_deposit();

        assert!(result.is_right());
    }

    #[rstest]
    fn can_deposit_frozen_returns_right() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;

        let result = account.can_deposit();

        assert!(result.is_right());
    }

    #[rstest]
    fn can_deposit_closed_returns_account_closed() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;

        let result = account.can_deposit();

        assert!(result.is_left());
        let error = result.unwrap_left();
        if let DomainError::AccountClosed(id) = error {
            assert_eq!(id, account.id);
        } else {
            panic!("Expected AccountClosed error");
        }
    }

    // =========================================================================
    // is_active, is_frozen, is_closed Tests
    // =========================================================================

    #[rstest]
    fn is_active_returns_true_for_active_account() {
        let account = create_active_account();
        assert!(account.is_active());
        assert!(!account.is_frozen());
        assert!(!account.is_closed());
    }

    #[rstest]
    fn is_frozen_returns_true_for_frozen_account() {
        let mut account = create_active_account();
        account.status = AccountStatus::Frozen;

        assert!(!account.is_active());
        assert!(account.is_frozen());
        assert!(!account.is_closed());
    }

    #[rstest]
    fn is_closed_returns_true_for_closed_account() {
        let mut account = create_active_account();
        account.status = AccountStatus::Closed;

        assert!(!account.is_active());
        assert!(!account.is_frozen());
        assert!(account.is_closed());
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[rstest]
    fn serialize_deserialize_account_roundtrip() {
        let account = create_active_account();
        let serialized = serde_json::to_string(&account).unwrap();
        let deserialized: Account = serde_json::from_str(&serialized).unwrap();

        assert_eq!(account, deserialized);
    }

    #[rstest]
    fn serialize_deserialize_account_status_roundtrip() {
        for status in [
            AccountStatus::Active,
            AccountStatus::Frozen,
            AccountStatus::Closed,
        ] {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: AccountStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[rstest]
    fn clone_produces_equal_account() {
        let account = create_active_account();
        let cloned = account.clone();

        assert_eq!(account, cloned);
    }
}
