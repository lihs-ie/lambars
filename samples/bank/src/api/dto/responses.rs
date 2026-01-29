//! Response DTOs for the Bank API.
//!
//! These DTOs represent outgoing HTTP response bodies.
//! They are serialized to JSON for API responses.

use serde::Serialize;

/// DTO for monetary values in responses.
///
/// Uses string representation for decimal amount to maintain
/// precision in JSON serialization.
///
/// # Example JSON
///
/// ```json
/// {
///     "amount": "10000",
///     "currency": "JPY"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MoneyResponseDto {
    /// The monetary amount as a decimal string.
    pub amount: String,
    /// The currency code (e.g., "JPY", "USD", "EUR").
    pub currency: String,
}

/// Response DTO for account information.
///
/// # Example JSON
///
/// ```json
/// {
///     "account_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "owner_name": "Alice",
///     "balance": {
///         "amount": "10000",
///         "currency": "JPY"
///     },
///     "status": "Active"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountResponse {
    /// The unique account identifier.
    pub account_id: String,
    /// The name of the account owner.
    pub owner_name: String,
    /// The current balance.
    pub balance: MoneyResponseDto,
    /// The account status ("Active", "Frozen", or "Closed").
    pub status: String,
}

/// Response DTO for a completed transaction.
///
/// # Example JSON
///
/// ```json
/// {
///     "transaction_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "amount": {
///         "amount": "5000",
///         "currency": "JPY"
///     },
///     "balance_after": {
///         "amount": "15000",
///         "currency": "JPY"
///     },
///     "timestamp": "2024-01-15T10:30:00Z"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransactionResponse {
    /// The unique transaction identifier.
    pub transaction_id: String,
    /// The transaction amount.
    pub amount: MoneyResponseDto,
    /// The account balance after the transaction.
    pub balance_after: MoneyResponseDto,
    /// The timestamp when the transaction occurred.
    pub timestamp: String,
}

/// Response DTO for balance inquiry.
///
/// # Example JSON
///
/// ```json
/// {
///     "account_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "balance": {
///         "amount": "10000",
///         "currency": "JPY"
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BalanceResponse {
    /// The account identifier.
    pub account_id: String,
    /// The current balance.
    pub balance: MoneyResponseDto,
}

/// DTO for a single transaction record in history.
///
/// # Example JSON
///
/// ```json
/// {
///     "transaction_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "transaction_type": "Deposit",
///     "amount": {
///         "amount": "5000",
///         "currency": "JPY"
///     },
///     "balance_after": {
///         "amount": "15000",
///         "currency": "JPY"
///     },
///     "counterparty_account_id": null,
///     "timestamp": "2024-01-15T10:30:00Z"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransactionRecordDto {
    /// The unique transaction identifier.
    pub transaction_id: String,
    /// The type of transaction (`Deposit`, `Withdrawal`, `TransferSent`, `TransferReceived`).
    pub transaction_type: String,
    /// The transaction amount.
    pub amount: MoneyResponseDto,
    /// The balance after the transaction.
    pub balance_after: MoneyResponseDto,
    /// The counterparty account ID for transfers (None for deposits/withdrawals).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_account_id: Option<String>,
    /// The timestamp when the transaction occurred.
    pub timestamp: String,
}

/// Response DTO for paginated transaction history.
///
/// # Example JSON
///
/// ```json
/// {
///     "account_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "transactions": [...],
///     "total": 100,
///     "page": 1,
///     "page_size": 20
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransactionHistoryResponse {
    /// The account identifier.
    pub account_id: String,
    /// The list of transactions for the current page.
    pub transactions: Vec<TransactionRecordDto>,
    /// The total number of transactions.
    pub total: usize,
    /// The current page number (1-indexed).
    pub page: usize,
    /// The number of items per page.
    pub page_size: usize,
}

/// Response DTO for transfer operation.
///
/// # Example JSON
///
/// ```json
/// {
///     "transfer_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "from_account_id": "...",
///     "to_account_id": "...",
///     "amount": {
///         "amount": "2000",
///         "currency": "JPY"
///     },
///     "from_balance_after": {...},
///     "timestamp": "2024-01-15T10:30:00Z"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransferResponse {
    /// The unique transfer transaction identifier.
    pub transfer_id: String,
    /// The source account ID.
    pub from_account_id: String,
    /// The destination account ID.
    pub to_account_id: String,
    /// The transfer amount.
    pub amount: MoneyResponseDto,
    /// The source account balance after the transfer.
    pub from_balance_after: MoneyResponseDto,
    /// The timestamp when the transfer occurred.
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // MoneyResponseDto Tests
    // =========================================================================

    #[rstest]
    fn money_response_dto_serializes_to_json() {
        let dto = MoneyResponseDto {
            amount: "10000".to_string(),
            currency: "JPY".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        assert!(json.contains("\"amount\":\"10000\""));
        assert!(json.contains("\"currency\":\"JPY\""));
    }

    #[rstest]
    fn money_response_dto_clone_produces_equal() {
        let dto = MoneyResponseDto {
            amount: "100".to_string(),
            currency: "USD".to_string(),
        };
        let cloned = dto.clone();

        assert_eq!(dto, cloned);
    }

    // =========================================================================
    // AccountResponse Tests
    // =========================================================================

    #[rstest]
    fn account_response_serializes_to_json() {
        let response = AccountResponse {
            account_id: "test-id".to_string(),
            owner_name: "Alice".to_string(),
            balance: MoneyResponseDto {
                amount: "10000".to_string(),
                currency: "JPY".to_string(),
            },
            status: "Active".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"account_id\":\"test-id\""));
        assert!(json.contains("\"owner_name\":\"Alice\""));
        assert!(json.contains("\"status\":\"Active\""));
    }

    // =========================================================================
    // TransactionResponse Tests
    // =========================================================================

    #[rstest]
    fn transaction_response_serializes_to_json() {
        let response = TransactionResponse {
            transaction_id: "tx-123".to_string(),
            amount: MoneyResponseDto {
                amount: "5000".to_string(),
                currency: "JPY".to_string(),
            },
            balance_after: MoneyResponseDto {
                amount: "15000".to_string(),
                currency: "JPY".to_string(),
            },
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"transaction_id\":\"tx-123\""));
        assert!(json.contains("\"timestamp\":\"2024-01-15T10:30:00Z\""));
    }

    // =========================================================================
    // BalanceResponse Tests
    // =========================================================================

    #[rstest]
    fn balance_response_serializes_to_json() {
        let response = BalanceResponse {
            account_id: "acc-123".to_string(),
            balance: MoneyResponseDto {
                amount: "10000".to_string(),
                currency: "JPY".to_string(),
            },
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"account_id\":\"acc-123\""));
        assert!(json.contains("\"balance\":{"));
    }

    // =========================================================================
    // TransactionRecordDto Tests
    // =========================================================================

    #[rstest]
    fn transaction_record_dto_serializes_without_counterparty() {
        let record = TransactionRecordDto {
            transaction_id: "tx-123".to_string(),
            transaction_type: "Deposit".to_string(),
            amount: MoneyResponseDto {
                amount: "5000".to_string(),
                currency: "JPY".to_string(),
            },
            balance_after: MoneyResponseDto {
                amount: "15000".to_string(),
                currency: "JPY".to_string(),
            },
            counterparty_account_id: None,
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();

        assert!(!json.contains("counterparty_account_id"));
    }

    #[rstest]
    fn transaction_record_dto_serializes_with_counterparty() {
        let record = TransactionRecordDto {
            transaction_id: "tx-123".to_string(),
            transaction_type: "TransferSent".to_string(),
            amount: MoneyResponseDto {
                amount: "2000".to_string(),
                currency: "JPY".to_string(),
            },
            balance_after: MoneyResponseDto {
                amount: "8000".to_string(),
                currency: "JPY".to_string(),
            },
            counterparty_account_id: Some("other-acc".to_string()),
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();

        assert!(json.contains("\"counterparty_account_id\":\"other-acc\""));
    }

    // =========================================================================
    // TransactionHistoryResponse Tests
    // =========================================================================

    #[rstest]
    fn transaction_history_response_serializes_to_json() {
        let response = TransactionHistoryResponse {
            account_id: "acc-123".to_string(),
            transactions: vec![],
            total: 100,
            page: 1,
            page_size: 20,
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"total\":100"));
        assert!(json.contains("\"page\":1"));
        assert!(json.contains("\"page_size\":20"));
    }

    // =========================================================================
    // TransferResponse Tests
    // =========================================================================

    #[rstest]
    fn transfer_response_serializes_to_json() {
        let response = TransferResponse {
            transfer_id: "tx-123".to_string(),
            from_account_id: "acc-from".to_string(),
            to_account_id: "acc-to".to_string(),
            amount: MoneyResponseDto {
                amount: "2000".to_string(),
                currency: "JPY".to_string(),
            },
            from_balance_after: MoneyResponseDto {
                amount: "8000".to_string(),
                currency: "JPY".to_string(),
            },
            timestamp: "2024-01-15T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"transfer_id\":\"tx-123\""));
        assert!(json.contains("\"from_account_id\":\"acc-from\""));
        assert!(json.contains("\"to_account_id\":\"acc-to\""));
    }
}
