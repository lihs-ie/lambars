//! Transaction history query for retrieving account transaction records.
//!
//! This module provides pure functions to build transaction history
//! from account events using `PersistentTreeMap` for time-ordered sorting.
//!
//! # Design Principles
//!
//! - **Pure Functions**: All functions have no side effects
//! - **Type Safety**: Strong typing for all inputs and outputs
//! - **Immutability**: Uses persistent data structures
//! - **Time Ordering**: Uses `PersistentTreeMap` keyed by timestamp
//!
//! # Examples
//!
//! ```rust
//! use bank::application::queries::{
//!     build_transaction_history, event_to_transaction_record,
//!     GetHistoryQuery, TransactionType,
//! };
//! use bank::domain::account::events::{AccountEvent, MoneyDeposited, EventId};
//! use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp, TransactionId};
//! use lambars::persistent::PersistentList;
//!
//! // Create events
//! let event = AccountEvent::Deposited(MoneyDeposited {
//!     event_id: EventId::generate(),
//!     account_id: AccountId::generate(),
//!     transaction_id: TransactionId::generate(),
//!     amount: Money::new(10000, Currency::JPY),
//!     balance_after: Money::new(20000, Currency::JPY),
//!     deposited_at: Timestamp::now(),
//! });
//!
//! // Convert event to transaction record
//! let record = event_to_transaction_record(&event);
//! assert!(record.is_some());
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::account::events::{
    AccountEvent, MoneyDeposited, MoneyWithdrawn, TransferReceived, TransferSent,
};
use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use lambars::persistent::{PersistentList, PersistentTreeMap};
use lambars::typeclass::Foldable;

/// Input for the transaction history query.
///
/// Contains the account ID and pagination parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetHistoryQuery {
    /// The ID of the account to query.
    pub account_id: AccountId,
    /// The offset for pagination (number of records to skip).
    pub offset: usize,
    /// The maximum number of records to return.
    pub limit: usize,
}

impl GetHistoryQuery {
    /// Creates a new history query with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to query
    /// * `offset` - The number of records to skip
    /// * `limit` - The maximum number of records to return
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bank::application::queries::GetHistoryQuery;
    /// use bank::domain::value_objects::AccountId;
    ///
    /// let query = GetHistoryQuery::new(AccountId::generate(), 0, 10);
    /// ```
    #[must_use]
    pub const fn new(account_id: AccountId, offset: usize, limit: usize) -> Self {
        Self {
            account_id,
            offset,
            limit,
        }
    }

    /// Creates a query for the first page with default limit of 10.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to query
    #[must_use]
    pub const fn first_page(account_id: AccountId) -> Self {
        Self::new(account_id, 0, 10)
    }

    /// Creates a query with a specific page number (0-indexed).
    ///
    /// # Arguments
    ///
    /// * `account_id` - The ID of the account to query
    /// * `page` - The page number (0-indexed)
    /// * `page_size` - The number of records per page
    #[must_use]
    pub const fn page(account_id: AccountId, page: usize, page_size: usize) -> Self {
        Self::new(account_id, page * page_size, page_size)
    }
}

/// The type of transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionType {
    /// Money was deposited into the account.
    Deposit,
    /// Money was withdrawn from the account.
    Withdrawal,
    /// Money was sent to another account.
    TransferSent,
    /// Money was received from another account.
    TransferReceived,
}

impl TransactionType {
    /// Returns `true` if this is a credit (money coming in).
    #[must_use]
    pub const fn is_credit(&self) -> bool {
        matches!(self, Self::Deposit | Self::TransferReceived)
    }

    /// Returns `true` if this is a debit (money going out).
    #[must_use]
    pub const fn is_debit(&self) -> bool {
        matches!(self, Self::Withdrawal | Self::TransferSent)
    }
}

/// A record of a single transaction.
///
/// This is a read-optimized representation of a transaction event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// The unique identifier for this transaction.
    pub transaction_id: TransactionId,
    /// The type of transaction.
    pub transaction_type: TransactionType,
    /// The amount of the transaction.
    pub amount: Money,
    /// The balance after this transaction.
    pub balance_after: Money,
    /// When the transaction occurred.
    pub timestamp: Timestamp,
    /// The counterparty account (for transfers).
    pub counterparty: Option<AccountId>,
}

impl TransactionRecord {
    /// Creates a new transaction record for a deposit.
    #[must_use]
    pub const fn deposit(
        transaction_id: TransactionId,
        amount: Money,
        balance_after: Money,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            transaction_id,
            transaction_type: TransactionType::Deposit,
            amount,
            balance_after,
            timestamp,
            counterparty: None,
        }
    }

    /// Creates a new transaction record for a withdrawal.
    #[must_use]
    pub const fn withdrawal(
        transaction_id: TransactionId,
        amount: Money,
        balance_after: Money,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            transaction_id,
            transaction_type: TransactionType::Withdrawal,
            amount,
            balance_after,
            timestamp,
            counterparty: None,
        }
    }

    /// Creates a new transaction record for an outgoing transfer.
    #[must_use]
    pub const fn transfer_sent(
        transaction_id: TransactionId,
        amount: Money,
        balance_after: Money,
        timestamp: Timestamp,
        to_account_id: AccountId,
    ) -> Self {
        Self {
            transaction_id,
            transaction_type: TransactionType::TransferSent,
            amount,
            balance_after,
            timestamp,
            counterparty: Some(to_account_id),
        }
    }

    /// Creates a new transaction record for an incoming transfer.
    #[must_use]
    pub const fn transfer_received(
        transaction_id: TransactionId,
        amount: Money,
        balance_after: Money,
        timestamp: Timestamp,
        from_account_id: AccountId,
    ) -> Self {
        Self {
            transaction_id,
            transaction_type: TransactionType::TransferReceived,
            amount,
            balance_after,
            timestamp,
            counterparty: Some(from_account_id),
        }
    }

    /// Returns `true` if this is a credit transaction.
    #[must_use]
    pub const fn is_credit(&self) -> bool {
        self.transaction_type.is_credit()
    }

    /// Returns `true` if this is a debit transaction.
    #[must_use]
    pub const fn is_debit(&self) -> bool {
        self.transaction_type.is_debit()
    }
}

/// Response containing transaction history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionHistory {
    /// The account ID.
    pub account_id: AccountId,
    /// The list of transactions (ordered by timestamp, newest first).
    pub transactions: Vec<TransactionRecord>,
    /// The total number of transactions (before pagination).
    pub total: usize,
    /// The offset used for this query.
    pub offset: usize,
    /// The limit used for this query.
    pub limit: usize,
}

impl TransactionHistory {
    /// Creates a new transaction history response.
    #[must_use]
    pub const fn new(
        account_id: AccountId,
        transactions: Vec<TransactionRecord>,
        total: usize,
        offset: usize,
        limit: usize,
    ) -> Self {
        Self {
            account_id,
            transactions,
            total,
            offset,
            limit,
        }
    }

    /// Returns `true` if there are more pages available.
    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.offset + self.transactions.len() < self.total
    }

    /// Returns the current page number (0-indexed).
    #[must_use]
    pub const fn current_page(&self) -> usize {
        if self.limit == 0 {
            0
        } else {
            self.offset / self.limit
        }
    }

    /// Returns the total number of pages.
    #[must_use]
    pub const fn total_pages(&self) -> usize {
        if self.limit == 0 {
            if self.total == 0 { 0 } else { 1 }
        } else {
            self.total.div_ceil(self.limit)
        }
    }

    /// Returns `true` if the result is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Returns the number of transactions in this page.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.transactions.len()
    }
}

/// Converts an `AccountEvent` to a `TransactionRecord`.
///
/// This is a pure function that extracts transaction information from an event.
/// Returns `None` for events that don't represent transactions (e.g., `AccountOpened`, `AccountClosed`).
///
/// # Arguments
///
/// * `event` - The account event to convert
///
/// # Returns
///
/// * `Some(TransactionRecord)` for transaction events
/// * `None` for non-transaction events
///
/// # Examples
///
/// ```rust
/// use bank::application::queries::{event_to_transaction_record, TransactionType};
/// use bank::domain::account::events::{AccountEvent, MoneyDeposited, EventId};
/// use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp, TransactionId};
///
/// let deposited = AccountEvent::Deposited(MoneyDeposited {
///     event_id: EventId::generate(),
///     account_id: AccountId::generate(),
///     transaction_id: TransactionId::generate(),
///     amount: Money::new(10000, Currency::JPY),
///     balance_after: Money::new(20000, Currency::JPY),
///     deposited_at: Timestamp::now(),
/// });
///
/// let record = event_to_transaction_record(&deposited);
/// assert!(record.is_some());
/// assert_eq!(record.unwrap().transaction_type, TransactionType::Deposit);
/// ```
#[must_use]
pub fn event_to_transaction_record(event: &AccountEvent) -> Option<TransactionRecord> {
    match event {
        AccountEvent::Deposited(MoneyDeposited {
            transaction_id,
            amount,
            balance_after,
            deposited_at,
            ..
        }) => Some(TransactionRecord::deposit(
            *transaction_id,
            amount.clone(),
            balance_after.clone(),
            *deposited_at,
        )),

        AccountEvent::Withdrawn(MoneyWithdrawn {
            transaction_id,
            amount,
            balance_after,
            withdrawn_at,
            ..
        }) => Some(TransactionRecord::withdrawal(
            *transaction_id,
            amount.clone(),
            balance_after.clone(),
            *withdrawn_at,
        )),

        AccountEvent::TransferSent(TransferSent {
            transaction_id,
            to_account_id,
            amount,
            balance_after,
            sent_at,
            ..
        }) => Some(TransactionRecord::transfer_sent(
            *transaction_id,
            amount.clone(),
            balance_after.clone(),
            *sent_at,
            *to_account_id,
        )),

        AccountEvent::TransferReceived(TransferReceived {
            transaction_id,
            from_account_id,
            amount,
            balance_after,
            received_at,
            ..
        }) => Some(TransactionRecord::transfer_received(
            *transaction_id,
            amount.clone(),
            balance_after.clone(),
            *received_at,
            *from_account_id,
        )),

        // Non-transaction events
        AccountEvent::Opened(_) | AccountEvent::Closed(_) => None,
    }
}

/// Builds a transaction history from a list of events.
///
/// This is a pure function that:
/// 1. Filters events to only include transactions
/// 2. Converts events to transaction records
/// 3. Uses `PersistentTreeMap` to sort by timestamp
/// 4. Applies pagination (offset and limit)
/// 5. Returns transactions in reverse chronological order (newest first)
///
/// # Arguments
///
/// * `account_id` - The account ID for the history
/// * `events` - The list of account events
/// * `query` - The query parameters including offset and limit
///
/// # Returns
///
/// A `TransactionHistory` containing the paginated transaction records
///
/// # Examples
///
/// ```rust
/// use bank::application::queries::{build_transaction_history, GetHistoryQuery};
/// use bank::domain::account::events::{AccountEvent, AccountOpened, MoneyDeposited, EventId};
/// use bank::domain::value_objects::{AccountId, Money, Currency, Timestamp, TransactionId};
/// use lambars::persistent::PersistentList;
///
/// let account_id = AccountId::generate();
///
/// let opened = AccountEvent::Opened(AccountOpened {
///     event_id: EventId::generate(),
///     account_id,
///     owner_name: "Alice".to_string(),
///     initial_balance: Money::new(10000, Currency::JPY),
///     opened_at: Timestamp::now(),
/// });
///
/// let deposited = AccountEvent::Deposited(MoneyDeposited {
///     event_id: EventId::generate(),
///     account_id,
///     transaction_id: TransactionId::generate(),
///     amount: Money::new(5000, Currency::JPY),
///     balance_after: Money::new(15000, Currency::JPY),
///     deposited_at: Timestamp::now(),
/// });
///
/// let events = PersistentList::new().cons(deposited).cons(opened);
/// let query = GetHistoryQuery::new(account_id, 0, 10);
///
/// let history = build_transaction_history(account_id, &events, &query);
///
/// assert_eq!(history.total, 1); // Only deposit is a transaction
/// assert_eq!(history.transactions.len(), 1);
/// ```
#[must_use]
pub fn build_transaction_history(
    account_id: AccountId,
    events: &PersistentList<AccountEvent>,
    query: &GetHistoryQuery,
) -> TransactionHistory {
    // Step 1: Build a PersistentTreeMap keyed by (Timestamp, TransactionId) for uniqueness
    // Using a composite key ensures stable ordering even for events with identical timestamps
    let sorted_map: PersistentTreeMap<(Timestamp, TransactionId), TransactionRecord> = events
        .clone()
        .fold_left(PersistentTreeMap::new(), |map, event| {
            if let Some(record) = event_to_transaction_record(&event) {
                let key = (record.timestamp, record.transaction_id);
                map.insert(key, record)
            } else {
                map
            }
        });

    let total = sorted_map.len();

    // Step 2: Collect records, reverse for newest-first order, then apply pagination
    // Note: PersistentTreeMap's values() iterator doesn't implement DoubleEndedIterator,
    // so we need to collect first before reversing
    #[allow(clippy::needless_collect)]
    let all_records: Vec<TransactionRecord> = sorted_map.values().cloned().collect();
    let transactions: Vec<TransactionRecord> = all_records
        .into_iter()
        .rev()
        .skip(query.offset)
        .take(query.limit)
        .collect();

    TransactionHistory::new(account_id, transactions, total, query.offset, query.limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::events::{AccountClosed, AccountOpened, EventId};
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_deposit_event(
        account_id: AccountId,
        amount: i64,
        balance_after: i64,
        timestamp: Timestamp,
    ) -> AccountEvent {
        AccountEvent::Deposited(MoneyDeposited {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            amount: Money::new(amount, Currency::JPY),
            balance_after: Money::new(balance_after, Currency::JPY),
            deposited_at: timestamp,
        })
    }

    fn create_withdrawal_event(
        account_id: AccountId,
        amount: i64,
        balance_after: i64,
        timestamp: Timestamp,
    ) -> AccountEvent {
        AccountEvent::Withdrawn(MoneyWithdrawn {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            amount: Money::new(amount, Currency::JPY),
            balance_after: Money::new(balance_after, Currency::JPY),
            withdrawn_at: timestamp,
        })
    }

    fn create_transfer_sent_event(
        account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
        balance_after: i64,
        timestamp: Timestamp,
    ) -> AccountEvent {
        AccountEvent::TransferSent(TransferSent {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            to_account_id,
            amount: Money::new(amount, Currency::JPY),
            balance_after: Money::new(balance_after, Currency::JPY),
            sent_at: timestamp,
        })
    }

    fn create_transfer_received_event(
        account_id: AccountId,
        from_account_id: AccountId,
        amount: i64,
        balance_after: i64,
        timestamp: Timestamp,
    ) -> AccountEvent {
        AccountEvent::TransferReceived(TransferReceived {
            event_id: EventId::generate(),
            account_id,
            transaction_id: TransactionId::generate(),
            from_account_id,
            amount: Money::new(amount, Currency::JPY),
            balance_after: Money::new(balance_after, Currency::JPY),
            received_at: timestamp,
        })
    }

    fn create_opened_event(account_id: AccountId, timestamp: Timestamp) -> AccountEvent {
        AccountEvent::Opened(AccountOpened {
            event_id: EventId::generate(),
            account_id,
            owner_name: "Test User".to_string(),
            initial_balance: Money::new(10000, Currency::JPY),
            opened_at: timestamp,
        })
    }

    fn create_closed_event(account_id: AccountId, timestamp: Timestamp) -> AccountEvent {
        AccountEvent::Closed(AccountClosed {
            event_id: EventId::generate(),
            account_id,
            closed_at: timestamp,
            final_balance: Money::zero(Currency::JPY),
        })
    }

    // =========================================================================
    // GetHistoryQuery Tests
    // =========================================================================

    #[rstest]
    fn get_history_query_new_creates_query() {
        let account_id = AccountId::generate();
        let query = GetHistoryQuery::new(account_id, 10, 20);

        assert_eq!(query.account_id, account_id);
        assert_eq!(query.offset, 10);
        assert_eq!(query.limit, 20);
    }

    #[rstest]
    fn get_history_query_first_page_creates_default_query() {
        let account_id = AccountId::generate();
        let query = GetHistoryQuery::first_page(account_id);

        assert_eq!(query.account_id, account_id);
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, 10);
    }

    #[rstest]
    fn get_history_query_page_creates_correct_offset() {
        let account_id = AccountId::generate();

        let page0 = GetHistoryQuery::page(account_id, 0, 10);
        assert_eq!(page0.offset, 0);
        assert_eq!(page0.limit, 10);

        let page1 = GetHistoryQuery::page(account_id, 1, 10);
        assert_eq!(page1.offset, 10);
        assert_eq!(page1.limit, 10);

        let page2 = GetHistoryQuery::page(account_id, 2, 25);
        assert_eq!(page2.offset, 50);
        assert_eq!(page2.limit, 25);
    }

    #[rstest]
    fn get_history_query_clone_produces_equal() {
        let query = GetHistoryQuery::new(AccountId::generate(), 5, 15);
        let cloned = query.clone();

        assert_eq!(query, cloned);
    }

    #[rstest]
    fn get_history_query_serialize_deserialize_roundtrip() {
        let query = GetHistoryQuery::new(AccountId::generate(), 5, 15);
        let serialized = serde_json::to_string(&query).unwrap();
        let deserialized: GetHistoryQuery = serde_json::from_str(&serialized).unwrap();

        assert_eq!(query, deserialized);
    }

    // =========================================================================
    // TransactionType Tests
    // =========================================================================

    #[rstest]
    fn transaction_type_is_credit_for_deposit_and_transfer_received() {
        assert!(TransactionType::Deposit.is_credit());
        assert!(TransactionType::TransferReceived.is_credit());
        assert!(!TransactionType::Withdrawal.is_credit());
        assert!(!TransactionType::TransferSent.is_credit());
    }

    #[rstest]
    fn transaction_type_is_debit_for_withdrawal_and_transfer_sent() {
        assert!(TransactionType::Withdrawal.is_debit());
        assert!(TransactionType::TransferSent.is_debit());
        assert!(!TransactionType::Deposit.is_debit());
        assert!(!TransactionType::TransferReceived.is_debit());
    }

    #[rstest]
    fn transaction_type_serialize_deserialize_roundtrip() {
        for transaction_type in [
            TransactionType::Deposit,
            TransactionType::Withdrawal,
            TransactionType::TransferSent,
            TransactionType::TransferReceived,
        ] {
            let serialized = serde_json::to_string(&transaction_type).unwrap();
            let deserialized: TransactionType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(transaction_type, deserialized);
        }
    }

    // =========================================================================
    // TransactionRecord Tests
    // =========================================================================

    #[rstest]
    fn transaction_record_deposit_creates_correct_record() {
        let transaction_id = TransactionId::generate();
        let amount = Money::new(10000, Currency::JPY);
        let balance_after = Money::new(20000, Currency::JPY);
        let timestamp = Timestamp::now();

        let record = TransactionRecord::deposit(
            transaction_id,
            amount.clone(),
            balance_after.clone(),
            timestamp,
        );

        assert_eq!(record.transaction_id, transaction_id);
        assert_eq!(record.transaction_type, TransactionType::Deposit);
        assert_eq!(record.amount, amount);
        assert_eq!(record.balance_after, balance_after);
        assert_eq!(record.timestamp, timestamp);
        assert!(record.counterparty.is_none());
    }

    #[rstest]
    fn transaction_record_withdrawal_creates_correct_record() {
        let transaction_id = TransactionId::generate();
        let amount = Money::new(5000, Currency::JPY);
        let balance_after = Money::new(15000, Currency::JPY);
        let timestamp = Timestamp::now();

        let record = TransactionRecord::withdrawal(
            transaction_id,
            amount.clone(),
            balance_after.clone(),
            timestamp,
        );

        assert_eq!(record.transaction_id, transaction_id);
        assert_eq!(record.transaction_type, TransactionType::Withdrawal);
        assert_eq!(record.amount, amount);
        assert_eq!(record.balance_after, balance_after);
        assert!(record.counterparty.is_none());
    }

    #[rstest]
    fn transaction_record_transfer_sent_creates_correct_record() {
        let transaction_id = TransactionId::generate();
        let to_account_id = AccountId::generate();
        let amount = Money::new(3000, Currency::JPY);
        let balance_after = Money::new(17000, Currency::JPY);
        let timestamp = Timestamp::now();

        let record = TransactionRecord::transfer_sent(
            transaction_id,
            amount,
            balance_after,
            timestamp,
            to_account_id,
        );

        assert_eq!(record.transaction_type, TransactionType::TransferSent);
        assert_eq!(record.counterparty, Some(to_account_id));
    }

    #[rstest]
    fn transaction_record_transfer_received_creates_correct_record() {
        let transaction_id = TransactionId::generate();
        let from_account_id = AccountId::generate();
        let amount = Money::new(2000, Currency::JPY);
        let balance_after = Money::new(22000, Currency::JPY);
        let timestamp = Timestamp::now();

        let record = TransactionRecord::transfer_received(
            transaction_id,
            amount,
            balance_after,
            timestamp,
            from_account_id,
        );

        assert_eq!(record.transaction_type, TransactionType::TransferReceived);
        assert_eq!(record.counterparty, Some(from_account_id));
    }

    #[rstest]
    fn transaction_record_is_credit_and_is_debit() {
        let timestamp = Timestamp::now();

        let deposit = TransactionRecord::deposit(
            TransactionId::generate(),
            Money::new(1000, Currency::JPY),
            Money::new(1000, Currency::JPY),
            timestamp,
        );
        assert!(deposit.is_credit());
        assert!(!deposit.is_debit());

        let withdrawal = TransactionRecord::withdrawal(
            TransactionId::generate(),
            Money::new(1000, Currency::JPY),
            Money::new(1000, Currency::JPY),
            timestamp,
        );
        assert!(!withdrawal.is_credit());
        assert!(withdrawal.is_debit());
    }

    #[rstest]
    fn transaction_record_serialize_deserialize_roundtrip() {
        let record = TransactionRecord::deposit(
            TransactionId::generate(),
            Money::new(10000, Currency::JPY),
            Money::new(20000, Currency::JPY),
            Timestamp::now(),
        );

        let serialized = serde_json::to_string(&record).unwrap();
        let deserialized: TransactionRecord = serde_json::from_str(&serialized).unwrap();

        assert_eq!(record, deserialized);
    }

    // =========================================================================
    // TransactionHistory Tests
    // =========================================================================

    #[rstest]
    fn transaction_history_new_creates_history() {
        let account_id = AccountId::generate();
        let transactions = vec![];
        let history = TransactionHistory::new(account_id, transactions, 100, 10, 20);

        assert_eq!(history.account_id, account_id);
        assert_eq!(history.total, 100);
        assert_eq!(history.offset, 10);
        assert_eq!(history.limit, 20);
    }

    #[rstest]
    fn transaction_history_has_more_returns_true_when_more_pages() {
        let account_id = AccountId::generate();
        let transactions = vec![TransactionRecord::deposit(
            TransactionId::generate(),
            Money::new(1000, Currency::JPY),
            Money::new(1000, Currency::JPY),
            Timestamp::now(),
        )];

        let history = TransactionHistory::new(account_id, transactions, 100, 0, 10);
        assert!(history.has_more());
    }

    #[rstest]
    fn transaction_history_has_more_returns_false_when_last_page() {
        let account_id = AccountId::generate();
        let transactions = vec![TransactionRecord::deposit(
            TransactionId::generate(),
            Money::new(1000, Currency::JPY),
            Money::new(1000, Currency::JPY),
            Timestamp::now(),
        )];

        let history = TransactionHistory::new(account_id, transactions, 1, 0, 10);
        assert!(!history.has_more());
    }

    #[rstest]
    fn transaction_history_current_page_calculates_correctly() {
        let account_id = AccountId::generate();

        let page0 = TransactionHistory::new(account_id, vec![], 100, 0, 10);
        assert_eq!(page0.current_page(), 0);

        let page1 = TransactionHistory::new(account_id, vec![], 100, 10, 10);
        assert_eq!(page1.current_page(), 1);

        let page5 = TransactionHistory::new(account_id, vec![], 100, 50, 10);
        assert_eq!(page5.current_page(), 5);
    }

    #[rstest]
    fn transaction_history_total_pages_calculates_correctly() {
        let account_id = AccountId::generate();

        let history100 = TransactionHistory::new(account_id, vec![], 100, 0, 10);
        assert_eq!(history100.total_pages(), 10);

        let history95 = TransactionHistory::new(account_id, vec![], 95, 0, 10);
        assert_eq!(history95.total_pages(), 10);

        let history0 = TransactionHistory::new(account_id, vec![], 0, 0, 10);
        assert_eq!(history0.total_pages(), 0);

        let limit0 = TransactionHistory::new(account_id, vec![], 10, 0, 0);
        assert_eq!(limit0.total_pages(), 1);
    }

    #[rstest]
    fn transaction_history_is_empty_and_len() {
        let account_id = AccountId::generate();

        let empty = TransactionHistory::new(account_id, vec![], 0, 0, 10);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let non_empty = TransactionHistory::new(
            account_id,
            vec![TransactionRecord::deposit(
                TransactionId::generate(),
                Money::new(1000, Currency::JPY),
                Money::new(1000, Currency::JPY),
                Timestamp::now(),
            )],
            1,
            0,
            10,
        );
        assert!(!non_empty.is_empty());
        assert_eq!(non_empty.len(), 1);
    }

    #[rstest]
    fn transaction_history_serialize_deserialize_roundtrip() {
        let history = TransactionHistory::new(
            AccountId::generate(),
            vec![TransactionRecord::deposit(
                TransactionId::generate(),
                Money::new(1000, Currency::JPY),
                Money::new(1000, Currency::JPY),
                Timestamp::now(),
            )],
            1,
            0,
            10,
        );

        let serialized = serde_json::to_string(&history).unwrap();
        let deserialized: TransactionHistory = serde_json::from_str(&serialized).unwrap();

        assert_eq!(history, deserialized);
    }

    // =========================================================================
    // event_to_transaction_record Tests
    // =========================================================================

    #[rstest]
    fn event_to_transaction_record_converts_deposit() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_deposit_event(account_id, 10000, 20000, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.transaction_type, TransactionType::Deposit);
        assert_eq!(*record.amount.amount(), rust_decimal::Decimal::from(10000));
        assert_eq!(record.timestamp, timestamp);
        assert!(record.counterparty.is_none());
    }

    #[rstest]
    fn event_to_transaction_record_converts_withdrawal() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_withdrawal_event(account_id, 5000, 15000, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.transaction_type, TransactionType::Withdrawal);
        assert_eq!(*record.amount.amount(), rust_decimal::Decimal::from(5000));
    }

    #[rstest]
    fn event_to_transaction_record_converts_transfer_sent() {
        let account_id = AccountId::generate();
        let to_account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_transfer_sent_event(account_id, to_account_id, 3000, 17000, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.transaction_type, TransactionType::TransferSent);
        assert_eq!(record.counterparty, Some(to_account_id));
    }

    #[rstest]
    fn event_to_transaction_record_converts_transfer_received() {
        let account_id = AccountId::generate();
        let from_account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event =
            create_transfer_received_event(account_id, from_account_id, 2000, 22000, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_some());
        let record = record.unwrap();
        assert_eq!(record.transaction_type, TransactionType::TransferReceived);
        assert_eq!(record.counterparty, Some(from_account_id));
    }

    #[rstest]
    fn event_to_transaction_record_returns_none_for_opened() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_opened_event(account_id, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_none());
    }

    #[rstest]
    fn event_to_transaction_record_returns_none_for_closed() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_closed_event(account_id, timestamp);

        let record = event_to_transaction_record(&event);

        assert!(record.is_none());
    }

    // =========================================================================
    // build_transaction_history Tests
    // =========================================================================

    #[rstest]
    fn build_transaction_history_empty_events_returns_empty_history() {
        let account_id = AccountId::generate();
        let events: PersistentList<AccountEvent> = PersistentList::new();
        let query = GetHistoryQuery::new(account_id, 0, 10);

        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 0);
        assert!(history.transactions.is_empty());
    }

    #[rstest]
    fn build_transaction_history_filters_non_transaction_events() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let events = PersistentList::new()
            .cons(create_closed_event(account_id, timestamp))
            .cons(create_deposit_event(account_id, 5000, 15000, timestamp))
            .cons(create_opened_event(account_id, timestamp));

        let query = GetHistoryQuery::new(account_id, 0, 10);
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 1); // Only deposit
        assert_eq!(history.transactions.len(), 1);
        assert_eq!(
            history.transactions[0].transaction_type,
            TransactionType::Deposit
        );
    }

    #[rstest]
    fn build_transaction_history_sorts_by_timestamp_newest_first() {
        let account_id = AccountId::generate();

        // Create events with distinct timestamps (using fixed unix seconds)
        let timestamp1 = Timestamp::from_unix_seconds(1000).unwrap();
        let timestamp2 = Timestamp::from_unix_seconds(2000).unwrap();
        let timestamp3 = Timestamp::from_unix_seconds(3000).unwrap();

        let events = PersistentList::new()
            .cons(create_deposit_event(account_id, 1000, 11000, timestamp1)) // oldest
            .cons(create_withdrawal_event(account_id, 500, 10500, timestamp3)) // newest
            .cons(create_deposit_event(account_id, 2000, 13000, timestamp2)); // middle

        let query = GetHistoryQuery::new(account_id, 0, 10);
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 3);
        assert_eq!(history.transactions.len(), 3);

        // Should be sorted newest first
        assert_eq!(history.transactions[0].timestamp, timestamp3);
        assert_eq!(history.transactions[1].timestamp, timestamp2);
        assert_eq!(history.transactions[2].timestamp, timestamp1);
    }

    #[rstest]
    fn build_transaction_history_applies_offset() {
        let account_id = AccountId::generate();

        let timestamp1 = Timestamp::from_unix_seconds(1000).unwrap();
        let timestamp2 = Timestamp::from_unix_seconds(2000).unwrap();
        let timestamp3 = Timestamp::from_unix_seconds(3000).unwrap();

        let events = PersistentList::new()
            .cons(create_deposit_event(account_id, 1000, 11000, timestamp1))
            .cons(create_deposit_event(account_id, 2000, 13000, timestamp2))
            .cons(create_deposit_event(account_id, 3000, 16000, timestamp3));

        let query = GetHistoryQuery::new(account_id, 1, 10); // Skip first
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 3);
        assert_eq!(history.transactions.len(), 2);
        assert_eq!(history.offset, 1);

        // Skipped the newest (timestamp3), should start with timestamp2
        assert_eq!(history.transactions[0].timestamp, timestamp2);
    }

    #[rstest]
    fn build_transaction_history_applies_limit() {
        let account_id = AccountId::generate();

        let timestamp1 = Timestamp::from_unix_seconds(1000).unwrap();
        let timestamp2 = Timestamp::from_unix_seconds(2000).unwrap();
        let timestamp3 = Timestamp::from_unix_seconds(3000).unwrap();

        let events = PersistentList::new()
            .cons(create_deposit_event(account_id, 1000, 11000, timestamp1))
            .cons(create_deposit_event(account_id, 2000, 13000, timestamp2))
            .cons(create_deposit_event(account_id, 3000, 16000, timestamp3));

        let query = GetHistoryQuery::new(account_id, 0, 2); // Only get 2
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 3);
        assert_eq!(history.transactions.len(), 2);
        assert_eq!(history.limit, 2);

        // Should get the 2 newest
        assert_eq!(history.transactions[0].timestamp, timestamp3);
        assert_eq!(history.transactions[1].timestamp, timestamp2);
    }

    #[rstest]
    fn build_transaction_history_offset_and_limit_combined() {
        let account_id = AccountId::generate();

        // Create 5 events
        let timestamps: Vec<Timestamp> = (1..=5_i64)
            .map(|i| Timestamp::from_unix_seconds(i * 1000).unwrap())
            .collect();

        let mut events = PersistentList::new();
        for (i, ts) in timestamps.iter().enumerate() {
            let amount = i64::try_from(i + 1).unwrap() * 1000;
            let balance = i64::try_from((i + 1) * 10000).unwrap();
            events = events.cons(create_deposit_event(account_id, amount, balance, *ts));
        }

        // Get page 1 (second page) with page size 2
        let query = GetHistoryQuery::new(account_id, 2, 2);
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 5);
        assert_eq!(history.transactions.len(), 2);

        // After skipping 2 newest (ts5, ts4), should get ts3 and ts2
        assert_eq!(history.transactions[0].timestamp, timestamps[2]); // ts3
        assert_eq!(history.transactions[1].timestamp, timestamps[1]); // ts2
    }

    #[rstest]
    fn build_transaction_history_offset_beyond_total_returns_empty() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let events =
            PersistentList::new().cons(create_deposit_event(account_id, 1000, 11000, timestamp));

        let query = GetHistoryQuery::new(account_id, 100, 10); // Offset way beyond
        let history = build_transaction_history(account_id, &events, &query);

        assert_eq!(history.total, 1);
        assert!(history.transactions.is_empty());
    }

    // =========================================================================
    // Pure Function Property Tests
    // =========================================================================

    #[rstest]
    fn build_transaction_history_is_referentially_transparent() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();

        let events =
            PersistentList::new().cons(create_deposit_event(account_id, 1000, 11000, timestamp));

        let query = GetHistoryQuery::new(account_id, 0, 10);

        let history1 = build_transaction_history(account_id, &events, &query);
        let history2 = build_transaction_history(account_id, &events, &query);

        assert_eq!(history1, history2);
    }

    #[rstest]
    fn event_to_transaction_record_is_referentially_transparent() {
        let account_id = AccountId::generate();
        let timestamp = Timestamp::now();
        let event = create_deposit_event(account_id, 10000, 20000, timestamp);

        let record1 = event_to_transaction_record(&event);
        let record2 = event_to_transaction_record(&event);

        assert_eq!(record1, record2);
    }

    // =========================================================================
    // Debug Tests
    // =========================================================================

    #[rstest]
    fn get_history_query_debug_is_implemented() {
        let query = GetHistoryQuery::new(AccountId::generate(), 0, 10);
        let debug_output = format!("{query:?}");
        assert!(!debug_output.is_empty());
    }

    #[rstest]
    fn transaction_type_debug_is_implemented() {
        let transaction_type = TransactionType::Deposit;
        let debug_output = format!("{transaction_type:?}");
        assert!(!debug_output.is_empty());
    }

    #[rstest]
    fn transaction_record_debug_is_implemented() {
        let record = TransactionRecord::deposit(
            TransactionId::generate(),
            Money::new(1000, Currency::JPY),
            Money::new(1000, Currency::JPY),
            Timestamp::now(),
        );
        let debug_output = format!("{record:?}");
        assert!(!debug_output.is_empty());
    }

    #[rstest]
    fn transaction_history_debug_is_implemented() {
        let history = TransactionHistory::new(AccountId::generate(), vec![], 0, 0, 10);
        let debug_output = format!("{history:?}");
        assert!(!debug_output.is_empty());
    }
}
