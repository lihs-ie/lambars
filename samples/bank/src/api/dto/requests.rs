//! Request DTOs for the Bank API.
//!
//! These DTOs represent incoming HTTP request bodies.
//! They use string representations for flexibility and validation
//! is performed during transformation to domain types.

use serde::Deserialize;

/// DTO for monetary values in requests.
///
/// Uses string representation for decimal amount to avoid
/// floating-point precision issues in JSON parsing.
///
/// # Example JSON
///
/// ```json
/// {
///     "amount": "10000",
///     "currency": "JPY"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MoneyDto {
    /// The monetary amount as a decimal string.
    pub amount: String,
    /// The currency code (e.g., "JPY", "USD", "EUR").
    pub currency: String,
}

/// Request DTO for opening a new account.
///
/// # Example JSON
///
/// ```json
/// {
///     "owner_name": "Alice",
///     "initial_balance": {
///         "amount": "10000",
///         "currency": "JPY"
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OpenAccountRequest {
    /// The name of the account owner.
    pub owner_name: String,
    /// The initial balance to deposit when opening the account.
    pub initial_balance: MoneyDto,
}

/// Request DTO for depositing money into an account.
///
/// # Example JSON
///
/// ```json
/// {
///     "amount": {
///         "amount": "5000",
///         "currency": "JPY"
///     },
///     "idempotency_key": "deposit-123-abc"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DepositRequest {
    /// The amount to deposit.
    pub amount: MoneyDto,
    /// A unique key for idempotent request handling.
    pub idempotency_key: String,
}

/// Request DTO for withdrawing money from an account.
///
/// # Example JSON
///
/// ```json
/// {
///     "amount": {
///         "amount": "3000",
///         "currency": "JPY"
///     },
///     "idempotency_key": "withdraw-456-def"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WithdrawRequest {
    /// The amount to withdraw.
    pub amount: MoneyDto,
    /// A unique key for idempotent request handling.
    pub idempotency_key: String,
}

/// Request DTO for transferring money between accounts.
///
/// # Example JSON
///
/// ```json
/// {
///     "to_account_id": "01234567-89ab-cdef-0123-456789abcdef",
///     "amount": {
///         "amount": "2000",
///         "currency": "JPY"
///     },
///     "idempotency_key": "transfer-789-ghi"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TransferRequest {
    /// The destination account ID.
    pub to_account_id: String,
    /// The amount to transfer.
    pub amount: MoneyDto,
    /// A unique key for idempotent request handling.
    pub idempotency_key: String,
}

/// Query parameters for paginated transaction history.
///
/// # Example Query String
///
/// ```text
/// ?page=1&page_size=20
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PaginationParams {
    /// The page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: usize,
    /// The number of items per page.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

/// Default page number (1).
const fn default_page() -> usize {
    1
}

/// Default page size (20).
const fn default_page_size() -> usize {
    20
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // =========================================================================
    // MoneyDto Tests
    // =========================================================================

    #[rstest]
    fn money_dto_deserializes_from_json() {
        let json = r#"{"amount": "10000", "currency": "JPY"}"#;
        let dto: MoneyDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.amount, "10000");
        assert_eq!(dto.currency, "JPY");
    }

    #[rstest]
    fn money_dto_deserializes_decimal_amount() {
        let json = r#"{"amount": "10.50", "currency": "USD"}"#;
        let dto: MoneyDto = serde_json::from_str(json).unwrap();

        assert_eq!(dto.amount, "10.50");
        assert_eq!(dto.currency, "USD");
    }

    #[rstest]
    fn money_dto_clone_produces_equal() {
        let dto = MoneyDto {
            amount: "100".to_string(),
            currency: "EUR".to_string(),
        };
        let cloned = dto.clone();

        assert_eq!(dto, cloned);
    }

    // =========================================================================
    // OpenAccountRequest Tests
    // =========================================================================

    #[rstest]
    fn open_account_request_deserializes_from_json() {
        let json = r#"{
            "owner_name": "Alice",
            "initial_balance": {
                "amount": "10000",
                "currency": "JPY"
            }
        }"#;
        let request: OpenAccountRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.owner_name, "Alice");
        assert_eq!(request.initial_balance.amount, "10000");
        assert_eq!(request.initial_balance.currency, "JPY");
    }

    #[rstest]
    fn open_account_request_fails_with_missing_fields() {
        let json = r#"{"owner_name": "Alice"}"#;
        let result: Result<OpenAccountRequest, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    // =========================================================================
    // DepositRequest Tests
    // =========================================================================

    #[rstest]
    fn deposit_request_deserializes_from_json() {
        let json = r#"{
            "amount": {
                "amount": "5000",
                "currency": "JPY"
            },
            "idempotency_key": "deposit-123"
        }"#;
        let request: DepositRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.amount.amount, "5000");
        assert_eq!(request.amount.currency, "JPY");
        assert_eq!(request.idempotency_key, "deposit-123");
    }

    // =========================================================================
    // WithdrawRequest Tests
    // =========================================================================

    #[rstest]
    fn withdraw_request_deserializes_from_json() {
        let json = r#"{
            "amount": {
                "amount": "3000",
                "currency": "JPY"
            },
            "idempotency_key": "withdraw-456"
        }"#;
        let request: WithdrawRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.amount.amount, "3000");
        assert_eq!(request.amount.currency, "JPY");
        assert_eq!(request.idempotency_key, "withdraw-456");
    }

    // =========================================================================
    // TransferRequest Tests
    // =========================================================================

    #[rstest]
    fn transfer_request_deserializes_from_json() {
        let json = r#"{
            "to_account_id": "01234567-89ab-cdef-0123-456789abcdef",
            "amount": {
                "amount": "2000",
                "currency": "JPY"
            },
            "idempotency_key": "transfer-789"
        }"#;
        let request: TransferRequest = serde_json::from_str(json).unwrap();

        assert_eq!(
            request.to_account_id,
            "01234567-89ab-cdef-0123-456789abcdef"
        );
        assert_eq!(request.amount.amount, "2000");
        assert_eq!(request.amount.currency, "JPY");
        assert_eq!(request.idempotency_key, "transfer-789");
    }

    // =========================================================================
    // PaginationParams Tests
    // =========================================================================

    #[rstest]
    fn pagination_params_deserializes_from_json() {
        let json = r#"{"page": 2, "page_size": 50}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.page, 2);
        assert_eq!(params.page_size, 50);
    }

    #[rstest]
    fn pagination_params_uses_defaults() {
        let json = "{}";
        let params: PaginationParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[rstest]
    fn pagination_params_default_trait() {
        let params = PaginationParams::default();

        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[rstest]
    fn pagination_params_partial_defaults() {
        let json = r#"{"page": 5}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.page, 5);
        assert_eq!(params.page_size, 20);
    }
}
