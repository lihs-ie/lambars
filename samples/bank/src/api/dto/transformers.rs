//! DTO transformation functions (Anti-Corruption Layer).
//!
//! This module provides pure functions for converting between DTOs and domain types.
//! Following functional programming principles:
//!
//! - **Pure Functions**: No side effects, same input always produces same output
//! - **Referential Transparency**: Functions can be replaced with their results
//! - **Either for Validation**: Validation errors are returned as `Either::Left`
//! - **Bifunctor Pattern**: Error transformation uses `map_left`/`map_right` style
//!
//! # Examples
//!
//! ```rust,ignore
//! use bank::api::dto::transformers::{money_to_dto, dto_to_money};
//! use bank::domain::value_objects::{Money, Currency};
//!
//! // Domain to DTO (pure function)
//! let money = Money::new(10000, Currency::JPY);
//! let dto = money_to_dto(&money);
//!
//! // DTO to Domain (validation)
//! let result = dto_to_money(&dto);
//! assert!(result.is_right());
//! ```

use lambars::control::Either;

use crate::api::dto::requests::MoneyDto;
use crate::api::dto::responses::{
    AccountResponse, MoneyResponseDto, TransactionRecordDto, TransactionResponse, TransferResponse,
};
use crate::domain::account::aggregate::{Account, AccountStatus};
use crate::domain::account::events::{AccountEvent, MoneyDeposited, MoneyWithdrawn, TransferSent};
use crate::domain::value_objects::{Currency, Money};

/// Validation errors that can occur during DTO transformation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformationError {
    /// The amount string could not be parsed as a valid decimal.
    InvalidAmount(String),
    /// The currency code is not recognized.
    InvalidCurrency(String),
}

impl std::fmt::Display for TransformationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAmount(value) => write!(formatter, "Invalid amount: {value}"),
            Self::InvalidCurrency(value) => write!(formatter, "Invalid currency: {value}"),
        }
    }
}

impl std::error::Error for TransformationError {}

// =============================================================================
// Money Transformations (Pure Functions)
// =============================================================================

/// Converts a domain `Money` to a response DTO.
///
/// This is a pure function that performs a simple structural transformation.
///
/// # Arguments
///
/// * `money` - The domain Money value
///
/// # Returns
///
/// A `MoneyResponseDto` with the amount as a string and currency code
///
/// # Examples
///
/// ```rust,ignore
/// let money = Money::new(10000, Currency::JPY);
/// let dto = money_to_dto(&money);
/// assert_eq!(dto.amount, "10000");
/// assert_eq!(dto.currency, "JPY");
/// ```
#[must_use]
pub fn money_to_dto(money: &Money) -> MoneyResponseDto {
    MoneyResponseDto {
        amount: money.amount().to_string(),
        currency: money.currency().to_string(),
    }
}

/// Parses a currency code string into a `Currency` enum.
///
/// This is a pure validation function.
///
/// # Arguments
///
/// * `currency_code` - A string like "JPY", "USD", or "EUR"
///
/// # Returns
///
/// * `Either::Right(Currency)` if the code is valid
/// * `Either::Left(TransformationError)` if the code is not recognized
fn parse_currency(currency_code: &str) -> Either<TransformationError, Currency> {
    match currency_code.to_uppercase().as_str() {
        "JPY" => Either::Right(Currency::JPY),
        "USD" => Either::Right(Currency::USD),
        "EUR" => Either::Right(Currency::EUR),
        _ => Either::Left(TransformationError::InvalidCurrency(
            currency_code.to_string(),
        )),
    }
}

/// Converts a request DTO to a domain `Money`.
///
/// This is a pure validation function that may fail if the amount or currency
/// is invalid.
///
/// # Arguments
///
/// * `dto` - The request `MoneyDto`
///
/// # Returns
///
/// * `Either::Right(Money)` if validation succeeds
/// * `Either::Left(TransformationError)` if validation fails
///
/// # Examples
///
/// ```rust,ignore
/// let dto = MoneyDto {
///     amount: "10000".to_string(),
///     currency: "JPY".to_string(),
/// };
/// let result = dto_to_money(&dto);
/// assert!(result.is_right());
/// ```
pub fn dto_to_money(dto: &MoneyDto) -> Either<TransformationError, Money> {
    // Parse currency first
    let currency = match parse_currency(&dto.currency) {
        Either::Right(currency) => currency,
        Either::Left(error) => return Either::Left(error),
    };

    // Parse amount
    Money::parse(&dto.amount, currency)
        .map_left(|_| TransformationError::InvalidAmount(dto.amount.clone()))
}

// =============================================================================
// Account Transformations (Pure Functions)
// =============================================================================

/// Converts an account status to its string representation.
///
/// This is a pure function.
#[must_use]
pub fn account_status_to_string(status: &AccountStatus) -> String {
    match status {
        AccountStatus::Active => "Active".to_string(),
        AccountStatus::Frozen => "Frozen".to_string(),
        AccountStatus::Closed => "Closed".to_string(),
    }
}

/// Converts a domain `Account` to a response DTO.
///
/// This is a pure function that performs structural transformation.
///
/// # Arguments
///
/// * `account` - The domain Account aggregate
///
/// # Returns
///
/// An `AccountResponse` DTO
///
/// # Examples
///
/// ```rust,ignore
/// let response = account_to_response(&account);
/// assert_eq!(response.owner_name, account.owner_name);
/// ```
#[must_use]
pub fn account_to_response(account: &Account) -> AccountResponse {
    AccountResponse {
        account_id: account.id.to_string(),
        owner_name: account.owner_name.clone(),
        balance: money_to_dto(&account.balance),
        status: account_status_to_string(&account.status),
    }
}

// =============================================================================
// Event Transformations (Pure Functions)
// =============================================================================

/// Converts a `MoneyDeposited` event to a transaction response DTO.
///
/// This is a pure function.
#[must_use]
pub fn deposit_event_to_transaction_response(event: &MoneyDeposited) -> TransactionResponse {
    TransactionResponse {
        transaction_id: event.transaction_id.to_string(),
        amount: money_to_dto(&event.amount),
        balance_after: money_to_dto(&event.balance_after),
        timestamp: event.deposited_at.to_iso_string(),
    }
}

/// Converts a `MoneyWithdrawn` event to a transaction response DTO.
///
/// This is a pure function.
#[must_use]
pub fn withdrawal_event_to_transaction_response(event: &MoneyWithdrawn) -> TransactionResponse {
    TransactionResponse {
        transaction_id: event.transaction_id.to_string(),
        amount: money_to_dto(&event.amount),
        balance_after: money_to_dto(&event.balance_after),
        timestamp: event.withdrawn_at.to_iso_string(),
    }
}

/// Converts a `TransferSent` event to a transfer response DTO.
///
/// This is a pure function.
#[must_use]
pub fn transfer_sent_event_to_response(event: &TransferSent) -> TransferResponse {
    TransferResponse {
        transfer_id: event.transaction_id.to_string(),
        from_account_id: event.account_id.to_string(),
        to_account_id: event.to_account_id.to_string(),
        amount: money_to_dto(&event.amount),
        from_balance_after: money_to_dto(&event.balance_after),
        timestamp: event.sent_at.to_iso_string(),
    }
}

/// Converts a generic transaction event to a transaction response DTO.
///
/// This function handles deposit and withdrawal events.
///
/// # Arguments
///
/// * `event` - The account event (must be Deposited or Withdrawn variant)
///
/// # Returns
///
/// * `Some(TransactionResponse)` for Deposited or Withdrawn events
/// * `None` for other event types
#[must_use]
pub fn event_to_transaction_response(event: &AccountEvent) -> Option<TransactionResponse> {
    match event {
        AccountEvent::Deposited(deposited) => {
            Some(deposit_event_to_transaction_response(deposited))
        }
        AccountEvent::Withdrawn(withdrawn) => {
            Some(withdrawal_event_to_transaction_response(withdrawn))
        }
        _ => None,
    }
}

/// Converts an account event to a transaction record DTO for history.
///
/// This is a pure function that handles all transaction-related events.
///
/// # Arguments
///
/// * `event` - Any account event
///
/// # Returns
///
/// * `Some(TransactionRecordDto)` for transaction events
/// * `None` for non-transaction events (Opened, Closed)
#[must_use]
pub fn event_to_transaction_record(event: &AccountEvent) -> Option<TransactionRecordDto> {
    match event {
        AccountEvent::Deposited(deposited) => Some(TransactionRecordDto {
            transaction_id: deposited.transaction_id.to_string(),
            transaction_type: "Deposit".to_string(),
            amount: money_to_dto(&deposited.amount),
            balance_after: money_to_dto(&deposited.balance_after),
            counterparty_account_id: None,
            timestamp: deposited.deposited_at.to_iso_string(),
        }),
        AccountEvent::Withdrawn(withdrawn) => Some(TransactionRecordDto {
            transaction_id: withdrawn.transaction_id.to_string(),
            transaction_type: "Withdrawal".to_string(),
            amount: money_to_dto(&withdrawn.amount),
            balance_after: money_to_dto(&withdrawn.balance_after),
            counterparty_account_id: None,
            timestamp: withdrawn.withdrawn_at.to_iso_string(),
        }),
        AccountEvent::TransferSent(sent) => Some(TransactionRecordDto {
            transaction_id: sent.transaction_id.to_string(),
            transaction_type: "TransferSent".to_string(),
            amount: money_to_dto(&sent.amount),
            balance_after: money_to_dto(&sent.balance_after),
            counterparty_account_id: Some(sent.to_account_id.to_string()),
            timestamp: sent.sent_at.to_iso_string(),
        }),
        AccountEvent::TransferReceived(received) => Some(TransactionRecordDto {
            transaction_id: received.transaction_id.to_string(),
            transaction_type: "TransferReceived".to_string(),
            amount: money_to_dto(&received.amount),
            balance_after: money_to_dto(&received.balance_after),
            counterparty_account_id: Some(received.from_account_id.to_string()),
            timestamp: received.received_at.to_iso_string(),
        }),
        AccountEvent::Opened(_) | AccountEvent::Closed(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::account::events::{AccountClosed, AccountOpened, EventId, TransferReceived};
    use crate::domain::value_objects::{AccountId, Timestamp, TransactionId};
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_test_money() -> Money {
        Money::new(10000, Currency::JPY)
    }

    fn create_test_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: create_test_money(),
            status: AccountStatus::Active,
            version: 1,
        }
    }

    fn create_test_deposit_event() -> MoneyDeposited {
        MoneyDeposited {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            amount: Money::new(5000, Currency::JPY),
            balance_after: Money::new(15000, Currency::JPY),
            deposited_at: Timestamp::now(),
        }
    }

    fn create_test_withdrawal_event() -> MoneyWithdrawn {
        MoneyWithdrawn {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            amount: Money::new(3000, Currency::JPY),
            balance_after: Money::new(7000, Currency::JPY),
            withdrawn_at: Timestamp::now(),
        }
    }

    fn create_test_transfer_sent_event() -> TransferSent {
        TransferSent {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            transaction_id: TransactionId::generate(),
            to_account_id: AccountId::generate(),
            amount: Money::new(2000, Currency::JPY),
            balance_after: Money::new(8000, Currency::JPY),
            sent_at: Timestamp::now(),
        }
    }

    fn create_test_transfer_received_event() -> TransferReceived {
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

    // =========================================================================
    // money_to_dto Tests
    // =========================================================================

    #[rstest]
    fn money_to_dto_converts_jpy() {
        let money = Money::new(10000, Currency::JPY);
        let dto = money_to_dto(&money);

        assert_eq!(dto.amount, "10000");
        assert_eq!(dto.currency, "JPY");
    }

    #[rstest]
    fn money_to_dto_converts_usd() {
        let money = Money::new(1050, Currency::USD);
        let dto = money_to_dto(&money);

        assert_eq!(dto.amount, "1050");
        assert_eq!(dto.currency, "USD");
    }

    #[rstest]
    fn money_to_dto_converts_eur() {
        let money = Money::new(999, Currency::EUR);
        let dto = money_to_dto(&money);

        assert_eq!(dto.amount, "999");
        assert_eq!(dto.currency, "EUR");
    }

    #[rstest]
    fn money_to_dto_is_pure() {
        let money = create_test_money();

        let dto1 = money_to_dto(&money);
        let dto2 = money_to_dto(&money);

        assert_eq!(dto1, dto2);
    }

    // =========================================================================
    // parse_currency Tests
    // =========================================================================

    #[rstest]
    fn parse_currency_jpy_returns_right() {
        let result = parse_currency("JPY");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Currency::JPY);
    }

    #[rstest]
    fn parse_currency_usd_returns_right() {
        let result = parse_currency("USD");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Currency::USD);
    }

    #[rstest]
    fn parse_currency_eur_returns_right() {
        let result = parse_currency("EUR");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Currency::EUR);
    }

    #[rstest]
    fn parse_currency_lowercase_returns_right() {
        let result = parse_currency("jpy");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Currency::JPY);
    }

    #[rstest]
    fn parse_currency_mixed_case_returns_right() {
        let result = parse_currency("Jpy");
        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), Currency::JPY);
    }

    #[rstest]
    fn parse_currency_invalid_returns_left() {
        let result = parse_currency("GBP");
        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            TransformationError::InvalidCurrency("GBP".to_string())
        );
    }

    // =========================================================================
    // dto_to_money Tests
    // =========================================================================

    #[rstest]
    fn dto_to_money_valid_returns_right() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_right());
        let money = result.unwrap_right();
        assert_eq!(money.amount().to_string(), "10000");
        assert_eq!(money.currency(), Currency::JPY);
    }

    #[rstest]
    fn dto_to_money_decimal_amount_returns_right() {
        let dto = MoneyDto {
            amount: "10.50".to_string(),
            currency: "USD".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_right());
        let money = result.unwrap_right();
        assert_eq!(money.amount().to_string(), "10.50");
    }

    #[rstest]
    fn dto_to_money_invalid_amount_returns_left() {
        let dto = MoneyDto {
            amount: "not-a-number".to_string(),
            currency: "JPY".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            TransformationError::InvalidAmount("not-a-number".to_string())
        );
    }

    #[rstest]
    fn dto_to_money_invalid_currency_returns_left() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "XYZ".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
        let error = result.unwrap_left();
        assert_eq!(
            error,
            TransformationError::InvalidCurrency("XYZ".to_string())
        );
    }

    #[rstest]
    fn dto_to_money_is_pure() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        let result1 = dto_to_money(&dto);
        let result2 = dto_to_money(&dto);

        assert_eq!(result1, result2);
    }

    // =========================================================================
    // account_status_to_string Tests
    // =========================================================================

    #[rstest]
    fn account_status_to_string_active() {
        assert_eq!(account_status_to_string(&AccountStatus::Active), "Active");
    }

    #[rstest]
    fn account_status_to_string_frozen() {
        assert_eq!(account_status_to_string(&AccountStatus::Frozen), "Frozen");
    }

    #[rstest]
    fn account_status_to_string_closed() {
        assert_eq!(account_status_to_string(&AccountStatus::Closed), "Closed");
    }

    // =========================================================================
    // account_to_response Tests
    // =========================================================================

    #[rstest]
    fn account_to_response_converts_correctly() {
        let account = create_test_account();
        let response = account_to_response(&account);

        assert_eq!(response.account_id, account.id.to_string());
        assert_eq!(response.owner_name, account.owner_name);
        assert_eq!(
            response.balance.amount,
            account.balance.amount().to_string()
        );
        assert_eq!(response.status, "Active");
    }

    #[rstest]
    fn account_to_response_is_pure() {
        let account = create_test_account();

        let response1 = account_to_response(&account);
        let response2 = account_to_response(&account);

        assert_eq!(response1, response2);
    }

    // =========================================================================
    // deposit_event_to_transaction_response Tests
    // =========================================================================

    #[rstest]
    fn deposit_event_to_transaction_response_converts_correctly() {
        let event = create_test_deposit_event();
        let response = deposit_event_to_transaction_response(&event);

        assert_eq!(response.transaction_id, event.transaction_id.to_string());
        assert_eq!(response.amount.amount, event.amount.amount().to_string());
        assert_eq!(
            response.balance_after.amount,
            event.balance_after.amount().to_string()
        );
    }

    // =========================================================================
    // withdrawal_event_to_transaction_response Tests
    // =========================================================================

    #[rstest]
    fn withdrawal_event_to_transaction_response_converts_correctly() {
        let event = create_test_withdrawal_event();
        let response = withdrawal_event_to_transaction_response(&event);

        assert_eq!(response.transaction_id, event.transaction_id.to_string());
        assert_eq!(response.amount.amount, event.amount.amount().to_string());
    }

    // =========================================================================
    // transfer_sent_event_to_response Tests
    // =========================================================================

    #[rstest]
    fn transfer_sent_event_to_response_converts_correctly() {
        let event = create_test_transfer_sent_event();
        let response = transfer_sent_event_to_response(&event);

        assert_eq!(response.transfer_id, event.transaction_id.to_string());
        assert_eq!(response.from_account_id, event.account_id.to_string());
        assert_eq!(response.to_account_id, event.to_account_id.to_string());
    }

    // =========================================================================
    // event_to_transaction_response Tests
    // =========================================================================

    #[rstest]
    fn event_to_transaction_response_deposited_returns_some() {
        let event = AccountEvent::Deposited(create_test_deposit_event());
        let result = event_to_transaction_response(&event);

        assert!(result.is_some());
    }

    #[rstest]
    fn event_to_transaction_response_withdrawn_returns_some() {
        let event = AccountEvent::Withdrawn(create_test_withdrawal_event());
        let result = event_to_transaction_response(&event);

        assert!(result.is_some());
    }

    #[rstest]
    fn event_to_transaction_response_other_events_returns_none() {
        let event = AccountEvent::TransferSent(create_test_transfer_sent_event());
        let result = event_to_transaction_response(&event);

        assert!(result.is_none());
    }

    // =========================================================================
    // event_to_transaction_record Tests
    // =========================================================================

    #[rstest]
    fn event_to_transaction_record_deposited_returns_some() {
        let deposit = create_test_deposit_event();
        let event = AccountEvent::Deposited(deposit);
        let result = event_to_transaction_record(&event);

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.transaction_type, "Deposit");
        assert!(record.counterparty_account_id.is_none());
    }

    #[rstest]
    fn event_to_transaction_record_withdrawn_returns_some() {
        let event = AccountEvent::Withdrawn(create_test_withdrawal_event());
        let result = event_to_transaction_record(&event);

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.transaction_type, "Withdrawal");
        assert!(record.counterparty_account_id.is_none());
    }

    #[rstest]
    fn event_to_transaction_record_transfer_sent_returns_some() {
        let sent = create_test_transfer_sent_event();
        let to_account = sent.to_account_id.to_string();
        let event = AccountEvent::TransferSent(sent);
        let result = event_to_transaction_record(&event);

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.transaction_type, "TransferSent");
        assert_eq!(record.counterparty_account_id, Some(to_account));
    }

    #[rstest]
    fn event_to_transaction_record_transfer_received_returns_some() {
        let received = create_test_transfer_received_event();
        let from_account = received.from_account_id.to_string();
        let event = AccountEvent::TransferReceived(received);
        let result = event_to_transaction_record(&event);

        assert!(result.is_some());
        let record = result.unwrap();
        assert_eq!(record.transaction_type, "TransferReceived");
        assert_eq!(record.counterparty_account_id, Some(from_account));
    }

    #[rstest]
    fn event_to_transaction_record_opened_returns_none() {
        let event = AccountEvent::Opened(AccountOpened {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            owner_name: "Test".to_string(),
            initial_balance: Money::new(10000, Currency::JPY),
            opened_at: Timestamp::now(),
        });
        let result = event_to_transaction_record(&event);

        assert!(result.is_none());
    }

    #[rstest]
    fn event_to_transaction_record_closed_returns_none() {
        let event = AccountEvent::Closed(AccountClosed {
            event_id: EventId::generate(),
            account_id: AccountId::generate(),
            closed_at: Timestamp::now(),
            final_balance: Money::zero(Currency::JPY),
        });
        let result = event_to_transaction_record(&event);

        assert!(result.is_none());
    }

    // =========================================================================
    // Roundtrip Tests (Iso-like behavior)
    // =========================================================================

    #[rstest]
    fn money_roundtrip_preserves_value() {
        let original = Money::new(12345, Currency::JPY);
        let dto = money_to_dto(&original);
        let request_dto = MoneyDto {
            amount: dto.amount,
            currency: dto.currency,
        };
        let result = dto_to_money(&request_dto);

        assert!(result.is_right());
        assert_eq!(result.unwrap_right(), original);
    }

    // =========================================================================
    // TransformationError Tests
    // =========================================================================

    #[rstest]
    fn transformation_error_display_invalid_amount() {
        let error = TransformationError::InvalidAmount("bad".to_string());
        assert_eq!(format!("{error}"), "Invalid amount: bad");
    }

    #[rstest]
    fn transformation_error_display_invalid_currency() {
        let error = TransformationError::InvalidCurrency("XYZ".to_string());
        assert_eq!(format!("{error}"), "Invalid currency: XYZ");
    }
}
