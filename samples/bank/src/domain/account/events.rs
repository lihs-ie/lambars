//! Domain events for the Account aggregate.
//!
//! This module defines all events that can occur on a bank account.
//! Events are immutable records of facts that have happened in the domain.
//!
//! # Design Principles
//!
//! - **Immutability**: Events are never modified after creation
//! - **Type Safety**: Each event type is a distinct struct with typed fields
//! - **Pattern Matching**: Prism optics provide type-safe access to event variants
//!
//! # Available Events
//!
//! - [`AccountOpened`] - A new account was opened
//! - [`MoneyDeposited`] - Money was deposited into an account
//! - [`MoneyWithdrawn`] - Money was withdrawn from an account
//! - [`TransferSent`] - Money was sent from this account to another
//! - [`TransferReceived`] - Money was received from another account
//! - [`AccountClosed`] - The account was closed
//!
//! # Prism Usage
//!
//! Each event variant can be accessed using the corresponding Prism:
//!
//! ```rust,ignore
//! use bank::domain::account::events::{AccountEvent, MoneyDeposited};
//! use lambars::optics::Prism;
//!
//! let event = AccountEvent::Deposited(/* MoneyDeposited instance */);
//! let prism = AccountEvent::deposited_prism();
//!
//! // Pattern match using preview
//! if let Some(deposited) = prism.preview(&event) {
//!     println!("Deposited: {:?}", deposited.amount);
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{AccountId, Money, Timestamp, TransactionId};
use lambars::optics::{FunctionPrism, Prism};

/// Unique identifier for events.
///
/// Each event has a unique ID for tracking and idempotency purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(uuid::Uuid);

impl EventId {
    /// Generates a new unique event ID using UUID v7.
    #[must_use]
    pub fn generate() -> Self {
        Self(uuid::Uuid::now_v7())
    }

    /// Creates an `EventId` from a UUID.
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Event raised when a new account is opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOpened {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The ID of the newly created account.
    pub account_id: AccountId,
    /// The name of the account owner.
    pub owner_name: String,
    /// The initial balance deposited when opening the account.
    pub initial_balance: Money,
    /// When the account was opened.
    pub opened_at: Timestamp,
}

impl AccountOpened {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "AccountOpened"
    }
}

/// Event raised when money is deposited into an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyDeposited {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The account receiving the deposit.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The amount deposited.
    pub amount: Money,
    /// The account balance after the deposit.
    pub balance_after: Money,
    /// When the deposit occurred.
    pub deposited_at: Timestamp,
}

impl MoneyDeposited {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "MoneyDeposited"
    }
}

/// Event raised when money is withdrawn from an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyWithdrawn {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The account from which money was withdrawn.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The amount withdrawn.
    pub amount: Money,
    /// The account balance after the withdrawal.
    pub balance_after: Money,
    /// When the withdrawal occurred.
    pub withdrawn_at: Timestamp,
}

impl MoneyWithdrawn {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "MoneyWithdrawn"
    }
}

/// Event raised when money is sent from this account to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferSent {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The account sending the money.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The destination account.
    pub to_account_id: AccountId,
    /// The amount transferred.
    pub amount: Money,
    /// The account balance after the transfer.
    pub balance_after: Money,
    /// When the transfer was sent.
    pub sent_at: Timestamp,
}

impl TransferSent {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "TransferSent"
    }
}

/// Event raised when money is received from another account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReceived {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The account receiving the money.
    pub account_id: AccountId,
    /// The transaction ID for idempotency.
    pub transaction_id: TransactionId,
    /// The source account.
    pub from_account_id: AccountId,
    /// The amount received.
    pub amount: Money,
    /// The account balance after receiving.
    pub balance_after: Money,
    /// When the transfer was received.
    pub received_at: Timestamp,
}

impl TransferReceived {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "TransferReceived"
    }
}

/// Event raised when an account is closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountClosed {
    /// Unique identifier for this event.
    pub event_id: EventId,
    /// The closed account's ID.
    pub account_id: AccountId,
    /// When the account was closed.
    pub closed_at: Timestamp,
    /// The final balance when the account was closed.
    pub final_balance: Money,
}

impl AccountClosed {
    /// Returns the event type as a string.
    #[must_use]
    pub const fn event_type() -> &'static str {
        "AccountClosed"
    }
}

/// All possible events that can occur on an account.
///
/// This is an algebraic data type (ADT) representing the sum of all account events.
/// Each variant wraps a specific event struct containing the event details.
///
/// # Prism Access
///
/// Use the static prism methods to safely access specific event variants:
///
/// ```rust
/// use bank::domain::account::events::AccountEvent;
/// use lambars::optics::Prism;
///
/// let prism = AccountEvent::deposited_prism();
/// // Use prism.preview() to pattern match
/// // Use prism.review() to construct events
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AccountEvent {
    /// A new account was opened.
    Opened(AccountOpened),
    /// Money was deposited into the account.
    Deposited(MoneyDeposited),
    /// Money was withdrawn from the account.
    Withdrawn(MoneyWithdrawn),
    /// Money was sent to another account.
    TransferSent(TransferSent),
    /// Money was received from another account.
    TransferReceived(TransferReceived),
    /// The account was closed.
    Closed(AccountClosed),
}

impl AccountEvent {
    /// Returns the event type as a string.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Match arms call non-const functions
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Opened(_) => AccountOpened::event_type(),
            Self::Deposited(_) => MoneyDeposited::event_type(),
            Self::Withdrawn(_) => MoneyWithdrawn::event_type(),
            Self::TransferSent(_) => TransferSent::event_type(),
            Self::TransferReceived(_) => TransferReceived::event_type(),
            Self::Closed(_) => AccountClosed::event_type(),
        }
    }

    /// Returns the unique event ID.
    #[must_use]
    pub const fn event_id(&self) -> &EventId {
        match self {
            Self::Opened(event) => &event.event_id,
            Self::Deposited(event) => &event.event_id,
            Self::Withdrawn(event) => &event.event_id,
            Self::TransferSent(event) => &event.event_id,
            Self::TransferReceived(event) => &event.event_id,
            Self::Closed(event) => &event.event_id,
        }
    }

    /// Returns the account ID associated with this event.
    #[must_use]
    pub const fn account_id(&self) -> &AccountId {
        match self {
            Self::Opened(event) => &event.account_id,
            Self::Deposited(event) => &event.account_id,
            Self::Withdrawn(event) => &event.account_id,
            Self::TransferSent(event) => &event.account_id,
            Self::TransferReceived(event) => &event.account_id,
            Self::Closed(event) => &event.account_id,
        }
    }

    /// Returns the timestamp when this event occurred.
    #[must_use]
    pub const fn occurred_at(&self) -> &Timestamp {
        match self {
            Self::Opened(event) => &event.opened_at,
            Self::Deposited(event) => &event.deposited_at,
            Self::Withdrawn(event) => &event.withdrawn_at,
            Self::TransferSent(event) => &event.sent_at,
            Self::TransferReceived(event) => &event.received_at,
            Self::Closed(event) => &event.closed_at,
        }
    }

    /// Creates a Prism for the `Opened` variant.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bank::domain::account::events::{AccountEvent, AccountOpened};
    /// use lambars::optics::Prism;
    ///
    /// let prism = AccountEvent::opened_prism();
    /// // preview returns Some for Opened variants, None otherwise
    /// ```
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn opened_prism() -> impl Prism<Self, AccountOpened> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::Opened(inner) => Some(inner),
                _ => None,
            },
            Self::Opened,
            |event: Self| match event {
                Self::Opened(inner) => Some(inner),
                _ => None,
            },
        )
    }

    /// Creates a Prism for the `Deposited` variant.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn deposited_prism() -> impl Prism<Self, MoneyDeposited> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::Deposited(inner) => Some(inner),
                _ => None,
            },
            Self::Deposited,
            |event: Self| match event {
                Self::Deposited(inner) => Some(inner),
                _ => None,
            },
        )
    }

    /// Creates a Prism for the `Withdrawn` variant.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn withdrawn_prism() -> impl Prism<Self, MoneyWithdrawn> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::Withdrawn(inner) => Some(inner),
                _ => None,
            },
            Self::Withdrawn,
            |event: Self| match event {
                Self::Withdrawn(inner) => Some(inner),
                _ => None,
            },
        )
    }

    /// Creates a Prism for the `TransferSent` variant.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn transfer_sent_prism() -> impl Prism<Self, TransferSent> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::TransferSent(inner) => Some(inner),
                _ => None,
            },
            Self::TransferSent,
            |event: Self| match event {
                Self::TransferSent(inner) => Some(inner),
                _ => None,
            },
        )
    }

    /// Creates a Prism for the `TransferReceived` variant.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn transfer_received_prism() -> impl Prism<Self, TransferReceived> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::TransferReceived(inner) => Some(inner),
                _ => None,
            },
            Self::TransferReceived,
            |event: Self| match event {
                Self::TransferReceived(inner) => Some(inner),
                _ => None,
            },
        )
    }

    /// Creates a Prism for the `Closed` variant.
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn closed_prism() -> impl Prism<Self, AccountClosed> {
        FunctionPrism::new(
            |event: &Self| match event {
                Self::Closed(inner) => Some(inner),
                _ => None,
            },
            Self::Closed,
            |event: Self| match event {
                Self::Closed(inner) => Some(inner),
                _ => None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_account_opened() -> AccountOpened {
        AccountOpened {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            initial_balance: Money::new(10000, Currency::JPY),
            opened_at: Timestamp::now(),
        }
    }

    fn create_money_deposited() -> MoneyDeposited {
        MoneyDeposited {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            amount: Money::new(5000, Currency::JPY),
            balance_after: Money::new(15000, Currency::JPY),
            deposited_at: Timestamp::now(),
        }
    }

    fn create_money_withdrawn() -> MoneyWithdrawn {
        MoneyWithdrawn {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            amount: Money::new(3000, Currency::JPY),
            balance_after: Money::new(12000, Currency::JPY),
            withdrawn_at: Timestamp::now(),
        }
    }

    fn create_transfer_sent() -> TransferSent {
        TransferSent {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            to_account_id: AccountId::generate(),
            amount: Money::new(2000, Currency::JPY),
            balance_after: Money::new(10000, Currency::JPY),
            sent_at: Timestamp::now(),
        }
    }

    fn create_transfer_received() -> TransferReceived {
        TransferReceived {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            from_account_id: AccountId::generate(),
            amount: Money::new(2000, Currency::JPY),
            balance_after: Money::new(12000, Currency::JPY),
            received_at: Timestamp::now(),
        }
    }

    fn create_account_closed() -> AccountClosed {
        AccountClosed {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            closed_at: Timestamp::now(),
            final_balance: Money::zero(Currency::JPY),
        }
    }

    // =========================================================================
    // EventId Tests
    // =========================================================================

    #[rstest]
    fn event_id_generate_creates_unique_ids() {
        let id1 = EventId::generate();
        let id2 = EventId::generate();

        assert_ne!(id1, id2);
    }

    #[rstest]
    fn event_id_generate_produces_v7_uuid() {
        let id = EventId::generate();

        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[rstest]
    fn event_id_display_formats_as_uuid() {
        let uuid = uuid::Uuid::now_v7();
        let id = EventId::from_uuid(uuid);

        assert_eq!(format!("{id}"), uuid.to_string());
    }

    #[rstest]
    fn event_id_serialization_roundtrip() {
        let original = EventId::generate();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: EventId = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // AccountOpened Tests
    // =========================================================================

    #[rstest]
    fn account_opened_event_type() {
        assert_eq!(AccountOpened::event_type(), "AccountOpened");
    }

    #[rstest]
    fn account_opened_serialization_roundtrip() {
        let original = create_account_opened();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AccountOpened = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn account_opened_clone_produces_equal() {
        let original = create_account_opened();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    // =========================================================================
    // MoneyDeposited Tests
    // =========================================================================

    #[rstest]
    fn money_deposited_event_type() {
        assert_eq!(MoneyDeposited::event_type(), "MoneyDeposited");
    }

    #[rstest]
    fn money_deposited_serialization_roundtrip() {
        let original = create_money_deposited();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: MoneyDeposited = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // MoneyWithdrawn Tests
    // =========================================================================

    #[rstest]
    fn money_withdrawn_event_type() {
        assert_eq!(MoneyWithdrawn::event_type(), "MoneyWithdrawn");
    }

    #[rstest]
    fn money_withdrawn_serialization_roundtrip() {
        let original = create_money_withdrawn();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: MoneyWithdrawn = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // TransferSent Tests
    // =========================================================================

    #[rstest]
    fn transfer_sent_event_type() {
        assert_eq!(TransferSent::event_type(), "TransferSent");
    }

    #[rstest]
    fn transfer_sent_serialization_roundtrip() {
        let original = create_transfer_sent();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: TransferSent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // TransferReceived Tests
    // =========================================================================

    #[rstest]
    fn transfer_received_event_type() {
        assert_eq!(TransferReceived::event_type(), "TransferReceived");
    }

    #[rstest]
    fn transfer_received_serialization_roundtrip() {
        let original = create_transfer_received();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: TransferReceived = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // AccountClosed Tests
    // =========================================================================

    #[rstest]
    fn account_closed_event_type() {
        assert_eq!(AccountClosed::event_type(), "AccountClosed");
    }

    #[rstest]
    fn account_closed_serialization_roundtrip() {
        let original = create_account_closed();
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AccountClosed = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    // =========================================================================
    // AccountEvent Tests
    // =========================================================================

    #[rstest]
    fn account_event_event_type_opened() {
        let event = AccountEvent::Opened(create_account_opened());
        assert_eq!(event.event_type(), "AccountOpened");
    }

    #[rstest]
    fn account_event_event_type_deposited() {
        let event = AccountEvent::Deposited(create_money_deposited());
        assert_eq!(event.event_type(), "MoneyDeposited");
    }

    #[rstest]
    fn account_event_event_type_withdrawn() {
        let event = AccountEvent::Withdrawn(create_money_withdrawn());
        assert_eq!(event.event_type(), "MoneyWithdrawn");
    }

    #[rstest]
    fn account_event_event_type_transfer_sent() {
        let event = AccountEvent::TransferSent(create_transfer_sent());
        assert_eq!(event.event_type(), "TransferSent");
    }

    #[rstest]
    fn account_event_event_type_transfer_received() {
        let event = AccountEvent::TransferReceived(create_transfer_received());
        assert_eq!(event.event_type(), "TransferReceived");
    }

    #[rstest]
    fn account_event_event_type_closed() {
        let event = AccountEvent::Closed(create_account_closed());
        assert_eq!(event.event_type(), "AccountClosed");
    }

    #[rstest]
    fn account_event_event_id() {
        let inner = create_account_opened();
        let expected_id = inner.event_id;
        let event = AccountEvent::Opened(inner);

        assert_eq!(*event.event_id(), expected_id);
    }

    #[rstest]
    fn account_event_account_id() {
        let inner = create_account_opened();
        let expected_id = inner.account_id;
        let event = AccountEvent::Opened(inner);

        assert_eq!(*event.account_id(), expected_id);
    }

    #[rstest]
    fn account_event_occurred_at_opened() {
        let inner = create_account_opened();
        let expected_timestamp = inner.opened_at;
        let event = AccountEvent::Opened(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_occurred_at_deposited() {
        let inner = create_money_deposited();
        let expected_timestamp = inner.deposited_at;
        let event = AccountEvent::Deposited(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_occurred_at_withdrawn() {
        let inner = create_money_withdrawn();
        let expected_timestamp = inner.withdrawn_at;
        let event = AccountEvent::Withdrawn(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_occurred_at_transfer_sent() {
        let inner = create_transfer_sent();
        let expected_timestamp = inner.sent_at;
        let event = AccountEvent::TransferSent(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_occurred_at_transfer_received() {
        let inner = create_transfer_received();
        let expected_timestamp = inner.received_at;
        let event = AccountEvent::TransferReceived(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_occurred_at_closed() {
        let inner = create_account_closed();
        let expected_timestamp = inner.closed_at;
        let event = AccountEvent::Closed(inner);

        assert_eq!(*event.occurred_at(), expected_timestamp);
    }

    #[rstest]
    fn account_event_serialization_roundtrip_opened() {
        let event = AccountEvent::Opened(create_account_opened());
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AccountEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event, deserialized);
    }

    #[rstest]
    fn account_event_serialization_roundtrip_deposited() {
        let event = AccountEvent::Deposited(create_money_deposited());
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AccountEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event, deserialized);
    }

    #[rstest]
    fn account_event_serialization_includes_type_tag() {
        let event = AccountEvent::Opened(create_account_opened());
        let serialized = serde_json::to_string(&event).unwrap();

        assert!(serialized.contains("\"type\":\"Opened\""));
        assert!(serialized.contains("\"data\":{"));
    }

    // =========================================================================
    // Prism Tests - Preview (Pattern Matching)
    // =========================================================================

    #[rstest]
    fn opened_prism_preview_returns_some_for_opened() {
        let inner = create_account_opened();
        let event = AccountEvent::Opened(inner.clone());
        let prism = AccountEvent::opened_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn opened_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Deposited(create_money_deposited());
        let prism = AccountEvent::opened_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn deposited_prism_preview_returns_some_for_deposited() {
        let inner = create_money_deposited();
        let event = AccountEvent::Deposited(inner.clone());
        let prism = AccountEvent::deposited_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn deposited_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::deposited_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn withdrawn_prism_preview_returns_some_for_withdrawn() {
        let inner = create_money_withdrawn();
        let event = AccountEvent::Withdrawn(inner.clone());
        let prism = AccountEvent::withdrawn_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn withdrawn_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::withdrawn_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn transfer_sent_prism_preview_returns_some_for_transfer_sent() {
        let inner = create_transfer_sent();
        let event = AccountEvent::TransferSent(inner.clone());
        let prism = AccountEvent::transfer_sent_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn transfer_sent_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::transfer_sent_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn transfer_received_prism_preview_returns_some_for_transfer_received() {
        let inner = create_transfer_received();
        let event = AccountEvent::TransferReceived(inner.clone());
        let prism = AccountEvent::transfer_received_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn transfer_received_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::transfer_received_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn closed_prism_preview_returns_some_for_closed() {
        let inner = create_account_closed();
        let event = AccountEvent::Closed(inner.clone());
        let prism = AccountEvent::closed_prism();

        let result = prism.preview(&event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), &inner);
    }

    #[rstest]
    fn closed_prism_preview_returns_none_for_other_variants() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::closed_prism();

        let result = prism.preview(&event);

        assert!(result.is_none());
    }

    // =========================================================================
    // Prism Tests - Review (Construction)
    // =========================================================================

    #[rstest]
    fn opened_prism_review_constructs_opened_event() {
        let inner = create_account_opened();
        let prism = AccountEvent::opened_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::Opened(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    #[rstest]
    fn deposited_prism_review_constructs_deposited_event() {
        let inner = create_money_deposited();
        let prism = AccountEvent::deposited_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::Deposited(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    #[rstest]
    fn withdrawn_prism_review_constructs_withdrawn_event() {
        let inner = create_money_withdrawn();
        let prism = AccountEvent::withdrawn_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::Withdrawn(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    #[rstest]
    fn transfer_sent_prism_review_constructs_transfer_sent_event() {
        let inner = create_transfer_sent();
        let prism = AccountEvent::transfer_sent_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::TransferSent(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    #[rstest]
    fn transfer_received_prism_review_constructs_transfer_received_event() {
        let inner = create_transfer_received();
        let prism = AccountEvent::transfer_received_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::TransferReceived(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    #[rstest]
    fn closed_prism_review_constructs_closed_event() {
        let inner = create_account_closed();
        let prism = AccountEvent::closed_prism();

        let event = prism.review(inner.clone());

        assert!(matches!(event, AccountEvent::Closed(_)));
        assert_eq!(prism.preview(&event), Some(&inner));
    }

    // =========================================================================
    // Prism Tests - Preview Owned
    // =========================================================================

    #[rstest]
    fn opened_prism_preview_owned_returns_some_for_opened() {
        let inner = create_account_opened();
        let event = AccountEvent::Opened(inner.clone());
        let prism = AccountEvent::opened_prism();

        let result = prism.preview_owned(event);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), inner);
    }

    #[rstest]
    fn opened_prism_preview_owned_returns_none_for_other_variants() {
        let event = AccountEvent::Deposited(create_money_deposited());
        let prism = AccountEvent::opened_prism();

        let result = prism.preview_owned(event);

        assert!(result.is_none());
    }

    // =========================================================================
    // Prism Law Tests
    // =========================================================================

    // Law 1: preview(review(value)) == Some(&value)
    #[rstest]
    fn opened_prism_preview_review_law() {
        let value = create_account_opened();
        let prism = AccountEvent::opened_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    #[rstest]
    fn deposited_prism_preview_review_law() {
        let value = create_money_deposited();
        let prism = AccountEvent::deposited_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    #[rstest]
    fn withdrawn_prism_preview_review_law() {
        let value = create_money_withdrawn();
        let prism = AccountEvent::withdrawn_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    #[rstest]
    fn transfer_sent_prism_preview_review_law() {
        let value = create_transfer_sent();
        let prism = AccountEvent::transfer_sent_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    #[rstest]
    fn transfer_received_prism_preview_review_law() {
        let value = create_transfer_received();
        let prism = AccountEvent::transfer_received_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    #[rstest]
    fn closed_prism_preview_review_law() {
        let value = create_account_closed();
        let prism = AccountEvent::closed_prism();

        let constructed = prism.review(value.clone());
        let previewed = prism.preview(&constructed);

        assert_eq!(previewed, Some(&value));
    }

    // Law 2: if preview(source).is_some(), then review(preview(source).unwrap().clone()) == source
    #[rstest]
    fn opened_prism_review_preview_law() {
        let inner = create_account_opened();
        let source = AccountEvent::Opened(inner);
        let prism = AccountEvent::opened_prism();

        if let Some(previewed) = prism.preview(&source) {
            let reconstructed = prism.review(previewed.clone());
            assert_eq!(reconstructed, source);
        }
    }

    #[rstest]
    fn deposited_prism_review_preview_law() {
        let inner = create_money_deposited();
        let source = AccountEvent::Deposited(inner);
        let prism = AccountEvent::deposited_prism();

        if let Some(previewed) = prism.preview(&source) {
            let reconstructed = prism.review(previewed.clone());
            assert_eq!(reconstructed, source);
        }
    }

    // =========================================================================
    // Prism Modify Tests
    // =========================================================================

    #[rstest]
    fn deposited_prism_modify_option_modifies_matching_variant() {
        let inner = create_money_deposited();
        let event = AccountEvent::Deposited(inner);
        let prism = AccountEvent::deposited_prism();

        let modified = prism.modify_option(event, |mut deposited| {
            deposited.amount = Money::new(9999, Currency::JPY);
            deposited
        });

        assert!(modified.is_some());
        let modified_event = modified.unwrap();
        if let Some(deposited) = AccountEvent::deposited_prism().preview(&modified_event) {
            assert_eq!(
                *deposited.amount.amount(),
                rust_decimal::Decimal::from(9999)
            );
        } else {
            panic!("Expected Deposited variant");
        }
    }

    #[rstest]
    fn deposited_prism_modify_option_returns_none_for_non_matching_variant() {
        let event = AccountEvent::Opened(create_account_opened());
        let prism = AccountEvent::deposited_prism();

        let modified = prism.modify_option(event, |mut deposited| {
            deposited.amount = Money::new(9999, Currency::JPY);
            deposited
        });

        assert!(modified.is_none());
    }

    #[rstest]
    fn deposited_prism_modify_or_identity_modifies_matching_variant() {
        let inner = create_money_deposited();
        let event = AccountEvent::Deposited(inner);
        let prism = AccountEvent::deposited_prism();

        let modified = prism.modify_or_identity(event, |mut deposited| {
            deposited.amount = Money::new(9999, Currency::JPY);
            deposited
        });

        if let Some(deposited) = AccountEvent::deposited_prism().preview(&modified) {
            assert_eq!(
                *deposited.amount.amount(),
                rust_decimal::Decimal::from(9999)
            );
        } else {
            panic!("Expected Deposited variant");
        }
    }

    #[rstest]
    fn deposited_prism_modify_or_identity_returns_original_for_non_matching_variant() {
        let original = AccountEvent::Opened(create_account_opened());
        let original_clone = original.clone();
        let prism = AccountEvent::deposited_prism();

        let result = prism.modify_or_identity(original, |mut deposited| {
            deposited.amount = Money::new(9999, Currency::JPY);
            deposited
        });

        assert_eq!(result, original_clone);
    }
}
