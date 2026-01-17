//! Messaging infrastructure for event publication.
//!
//! This module provides pure data structures and functions for converting
//! domain events to SQS messages. The actual SQS client interaction is
//! handled elsewhere (in `main.rs` or dedicated workers).
//!
//! # Design
//!
//! - **Pure data structures**: `EventMessage` is a plain data structure
//! - **Pure transformation functions**: `event_to_message` and `events_to_messages`
//!   are referentially transparent
//! - **Serde integration**: Messages can be serialized/deserialized for transport
//!
//! # Example
//!
//! ```rust,ignore
//! use bank::infrastructure::{event_to_message, events_to_messages};
//! use bank::domain::account::events::AccountEvent;
//!
//! let event = AccountEvent::Deposited(/* ... */);
//! let message = event_to_message(&event);
//!
//! // Send via SQS client...
//! ```

use serde::{Deserialize, Serialize};

use crate::domain::account::events::AccountEvent;

/// A message representation for SQS transport.
///
/// Contains all information needed to publish a domain event
/// to an SQS queue. This is a pure data structure that can be
/// serialized and sent via AWS SDK.
///
/// # Fields
///
/// - `event_id`: Unique identifier for the event (for deduplication)
/// - `event_type`: Type of the event (e.g., "Opened", "Deposited")
/// - `aggregate_id`: The account ID this event belongs to
/// - `payload`: The full event data as JSON
/// - `occurred_at`: ISO 8601 timestamp of when the event occurred
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMessage {
    /// Unique identifier for this event.
    ///
    /// Used for SQS message deduplication.
    pub event_id: String,
    /// The type of the event.
    ///
    /// Useful for filtering and routing messages.
    pub event_type: String,
    /// The ID of the aggregate this event belongs to.
    pub aggregate_id: String,
    /// The full event data as JSON.
    pub payload: serde_json::Value,
    /// When the event occurred (ISO 8601 format).
    pub occurred_at: String,
}

impl EventMessage {
    /// Creates a new `EventMessage`.
    ///
    /// # Arguments
    ///
    /// * `event_id` - Unique identifier for the event
    /// * `event_type` - Type of the event
    /// * `aggregate_id` - The aggregate ID this event belongs to
    /// * `payload` - The event data as JSON
    /// * `occurred_at` - When the event occurred (ISO 8601)
    #[must_use]
    pub const fn new(
        event_id: String,
        event_type: String,
        aggregate_id: String,
        payload: serde_json::Value,
        occurred_at: String,
    ) -> Self {
        Self {
            event_id,
            event_type,
            aggregate_id,
            payload,
            occurred_at,
        }
    }
}

/// Converts a domain event to an SQS message.
///
/// This is a pure function that transforms an `AccountEvent` into
/// an `EventMessage` suitable for SQS transport.
///
/// # Arguments
///
/// * `event` - The domain event to convert
///
/// # Returns
///
/// An `EventMessage` containing all necessary information for SQS transport.
///
/// # Panics
///
/// Panics if the event cannot be serialized to JSON. This should never
/// happen for valid `AccountEvent` instances since they derive `Serialize`.
///
/// # Example
///
/// ```rust,ignore
/// use bank::infrastructure::event_to_message;
/// use bank::domain::account::events::{AccountEvent, AccountOpened};
///
/// let event = AccountEvent::Opened(AccountOpened { /* ... */ });
/// let message = event_to_message(&event);
///
/// assert_eq!(message.event_type, "AccountOpened");
/// ```
#[must_use]
pub fn event_to_message(event: &AccountEvent) -> EventMessage {
    // Serialize the event to JSON
    // This should never fail for valid AccountEvent instances
    let payload = serde_json::to_value(event).expect("AccountEvent should be serializable");

    EventMessage {
        event_id: event.event_id().to_string(),
        event_type: event.event_type().to_string(),
        aggregate_id: event.account_id().to_string(),
        payload,
        occurred_at: event.occurred_at().to_iso_string(),
    }
}

/// Converts multiple domain events to SQS messages.
///
/// This is a pure function that maps over a slice of events
/// and converts each one to an `EventMessage`.
///
/// # Arguments
///
/// * `events` - The domain events to convert
///
/// # Returns
///
/// A `Vec<EventMessage>` containing the converted messages.
///
/// # Example
///
/// ```rust,ignore
/// use bank::infrastructure::events_to_messages;
/// use bank::domain::account::events::AccountEvent;
///
/// let events: Vec<AccountEvent> = vec![/* ... */];
/// let messages = events_to_messages(&events);
///
/// assert_eq!(messages.len(), events.len());
/// ```
#[must_use]
pub fn events_to_messages(events: &[AccountEvent]) -> Vec<EventMessage> {
    events.iter().map(event_to_message).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::events::{
        AccountClosed, AccountOpened, EventId, MoneyDeposited, MoneyWithdrawn, TransferReceived,
        TransferSent,
    };
    use crate::domain::value_objects::{AccountId, Currency, Money, Timestamp, TransactionId};
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
    // EventMessage Tests
    // =========================================================================

    #[rstest]
    fn event_message_new() {
        let payload = serde_json::json!({"test": "value"});
        let message = EventMessage::new(
            "event-123".to_string(),
            "TestEvent".to_string(),
            "agg-456".to_string(),
            payload.clone(),
            "2024-01-15T10:30:00Z".to_string(),
        );

        assert_eq!(message.event_id, "event-123");
        assert_eq!(message.event_type, "TestEvent");
        assert_eq!(message.aggregate_id, "agg-456");
        assert_eq!(message.payload, payload);
        assert_eq!(message.occurred_at, "2024-01-15T10:30:00Z");
    }

    #[rstest]
    fn event_message_clone() {
        let payload = serde_json::json!({"key": "value"});
        let original = EventMessage::new(
            "id".to_string(),
            "Type".to_string(),
            "agg".to_string(),
            payload,
            "2024-01-01T00:00:00Z".to_string(),
        );
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[rstest]
    fn event_message_debug() {
        let payload = serde_json::json!({});
        let message = EventMessage::new(
            "id".to_string(),
            "Type".to_string(),
            "agg".to_string(),
            payload,
            "2024-01-01T00:00:00Z".to_string(),
        );
        let debug_str = format!("{message:?}");

        assert!(debug_str.contains("EventMessage"));
        assert!(debug_str.contains("event_id"));
        assert!(debug_str.contains("event_type"));
    }

    #[rstest]
    fn event_message_serialization_roundtrip() {
        let payload = serde_json::json!({"nested": {"key": "value"}});
        let original = EventMessage::new(
            "event-id".to_string(),
            "EventType".to_string(),
            "aggregate-id".to_string(),
            payload,
            "2024-06-15T12:00:00Z".to_string(),
        );

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: EventMessage = serde_json::from_str(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[rstest]
    fn event_message_equality() {
        let payload1 = serde_json::json!({"key": "value"});
        let payload2 = serde_json::json!({"key": "value"});
        let payload3 = serde_json::json!({"key": "different"});

        let message1 = EventMessage::new(
            "id".to_string(),
            "Type".to_string(),
            "agg".to_string(),
            payload1,
            "2024-01-01T00:00:00Z".to_string(),
        );
        let message2 = EventMessage::new(
            "id".to_string(),
            "Type".to_string(),
            "agg".to_string(),
            payload2,
            "2024-01-01T00:00:00Z".to_string(),
        );
        let message3 = EventMessage::new(
            "id".to_string(),
            "Type".to_string(),
            "agg".to_string(),
            payload3,
            "2024-01-01T00:00:00Z".to_string(),
        );

        assert_eq!(message1, message2);
        assert_ne!(message1, message3);
    }

    // =========================================================================
    // event_to_message Tests
    // =========================================================================

    #[rstest]
    fn event_to_message_account_opened() {
        let inner = create_account_opened();
        let event = AccountEvent::Opened(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "AccountOpened");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.opened_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_money_deposited() {
        let inner = create_money_deposited();
        let event = AccountEvent::Deposited(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "MoneyDeposited");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.deposited_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_money_withdrawn() {
        let inner = create_money_withdrawn();
        let event = AccountEvent::Withdrawn(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "MoneyWithdrawn");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.withdrawn_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_transfer_sent() {
        let inner = create_transfer_sent();
        let event = AccountEvent::TransferSent(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "TransferSent");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.sent_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_transfer_received() {
        let inner = create_transfer_received();
        let event = AccountEvent::TransferReceived(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "TransferReceived");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.received_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_account_closed() {
        let inner = create_account_closed();
        let event = AccountEvent::Closed(inner.clone());
        let message = event_to_message(&event);

        assert_eq!(message.event_id, inner.event_id.to_string());
        assert_eq!(message.event_type, "AccountClosed");
        assert_eq!(message.aggregate_id, inner.account_id.to_string());
        assert_eq!(message.occurred_at, inner.closed_at.to_iso_string());
    }

    #[rstest]
    fn event_to_message_payload_contains_event_data() {
        let inner = create_account_opened();
        let event = AccountEvent::Opened(inner);
        let message = event_to_message(&event);

        // The payload should contain the serialized event
        let payload = &message.payload;
        assert!(payload.is_object());

        // Check that it's a tagged enum with "type" and "data"
        assert!(payload.get("type").is_some());
        assert!(payload.get("data").is_some());
    }

    #[rstest]
    fn event_to_message_is_pure_function() {
        // Calling the function multiple times with the same input
        // should produce equal results (referential transparency)
        let inner = create_money_deposited();
        let event = AccountEvent::Deposited(inner);

        let message1 = event_to_message(&event);
        let message2 = event_to_message(&event);

        assert_eq!(message1.event_id, message2.event_id);
        assert_eq!(message1.event_type, message2.event_type);
        assert_eq!(message1.aggregate_id, message2.aggregate_id);
        assert_eq!(message1.payload, message2.payload);
        assert_eq!(message1.occurred_at, message2.occurred_at);
    }

    // =========================================================================
    // events_to_messages Tests
    // =========================================================================

    #[rstest]
    fn events_to_messages_empty_list() {
        let events: Vec<AccountEvent> = vec![];
        let messages = events_to_messages(&events);

        assert!(messages.is_empty());
    }

    #[rstest]
    fn events_to_messages_single_event() {
        let event = AccountEvent::Opened(create_account_opened());
        let events = vec![event];
        let messages = events_to_messages(&events);

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].event_type, "AccountOpened");
    }

    #[rstest]
    fn events_to_messages_multiple_events() {
        let events = vec![
            AccountEvent::Opened(create_account_opened()),
            AccountEvent::Deposited(create_money_deposited()),
            AccountEvent::Withdrawn(create_money_withdrawn()),
        ];
        let messages = events_to_messages(&events);

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].event_type, "AccountOpened");
        assert_eq!(messages[1].event_type, "MoneyDeposited");
        assert_eq!(messages[2].event_type, "MoneyWithdrawn");
    }

    #[rstest]
    fn events_to_messages_preserves_order() {
        let opened = create_account_opened();
        let deposited = create_money_deposited();
        let closed = create_account_closed();

        let events = vec![
            AccountEvent::Opened(opened.clone()),
            AccountEvent::Deposited(deposited.clone()),
            AccountEvent::Closed(closed.clone()),
        ];
        let messages = events_to_messages(&events);

        assert_eq!(messages[0].event_id, opened.event_id.to_string());
        assert_eq!(messages[1].event_id, deposited.event_id.to_string());
        assert_eq!(messages[2].event_id, closed.event_id.to_string());
    }

    #[rstest]
    fn events_to_messages_is_pure_function() {
        let events = vec![
            AccountEvent::Opened(create_account_opened()),
            AccountEvent::Deposited(create_money_deposited()),
        ];

        let messages1 = events_to_messages(&events);
        let messages2 = events_to_messages(&events);

        assert_eq!(messages1.len(), messages2.len());
        for (m1, m2) in messages1.iter().zip(messages2.iter()) {
            assert_eq!(m1.event_id, m2.event_id);
            assert_eq!(m1.event_type, m2.event_type);
        }
    }

    #[rstest]
    fn events_to_messages_all_event_types() {
        let events = vec![
            AccountEvent::Opened(create_account_opened()),
            AccountEvent::Deposited(create_money_deposited()),
            AccountEvent::Withdrawn(create_money_withdrawn()),
            AccountEvent::TransferSent(create_transfer_sent()),
            AccountEvent::TransferReceived(create_transfer_received()),
            AccountEvent::Closed(create_account_closed()),
        ];
        let messages = events_to_messages(&events);

        assert_eq!(messages.len(), 6);

        let expected_types = [
            "AccountOpened",
            "MoneyDeposited",
            "MoneyWithdrawn",
            "TransferSent",
            "TransferReceived",
            "AccountClosed",
        ];

        for (message, expected_type) in messages.iter().zip(expected_types.iter()) {
            assert_eq!(message.event_type, *expected_type);
        }
    }
}
