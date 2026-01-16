//! Transaction-related HTTP handlers.
//!
//! This module provides handlers for transaction operations:
//!
//! - `POST /accounts/{id}/deposit` - Deposit money
//! - `POST /accounts/{id}/withdraw` - Withdraw money
//! - `POST /transfers` - Transfer money between accounts
//! - `GET /accounts/{id}/transactions` - Get transaction history
//!
//! # Functional Design
//!
//! Handlers follow a pipeline pattern similar to account handlers.
//! Idempotency is handled via transaction IDs derived from idempotency keys.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use crate::api::dto::requests::{
    DepositRequest, PaginationParams, TransferRequest, WithdrawRequest,
};
use crate::api::dto::responses::{
    TransactionHistoryResponse, TransactionResponse, TransferResponse,
};
use crate::api::dto::transformers::dto_to_money;
use crate::api::middleware::error_handler::{
    ApiError, ApiErrorResponse, account_id_error_to_api_error, transformation_error_to_api_error,
};
use crate::domain::value_objects::{AccountId, TransactionId};
use crate::infrastructure::AppDependencies;

/// POST /accounts/{id}/deposit - Deposit money into an account.
///
/// Deposits the specified amount into the account.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Path Parameters
///
/// - `id` - The account UUID
///
/// # Request Body
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
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is not a valid UUID
/// - The amount or currency is invalid
/// - The account is not found
/// - The account is closed
///
/// # Response
///
/// - `201 Created` - Deposit successful
/// - `400 Bad Request` - Invalid request data
/// - `404 Not Found` - Account not found
/// - `409 Conflict` - Account is closed
///
/// # Example Response
///
/// ```json
/// {
///     "transaction_id": "...",
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
#[allow(clippy::unused_async)]
pub async fn deposit(
    State(_dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
    Json(request): Json<DepositRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiErrorResponse> {
    // Step 1: Validate and parse account ID (pure function)
    let account_id = AccountId::create(&account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let account_id = match account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Transform DTO to domain types (pure function)
    let amount = dto_to_money(&request.amount).map_left(|error| {
        let (status, api_error) = transformation_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let amount = match amount {
        lambars::control::Either::Right(amount) => amount,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 3: Create transaction ID from idempotency key (pure function)
    let transaction_id = TransactionId::from_idempotency_key(&request.idempotency_key);

    // Step 4: Execute workflow via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the account loaded

    // Mock: Return not found for demonstration
    let _ = (account_id, amount, transaction_id);
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({
                "account_id": account_id_string
            }),
        ),
    ))
}

/// POST /accounts/{id}/withdraw - Withdraw money from an account.
///
/// Withdraws the specified amount from the account.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Path Parameters
///
/// - `id` - The account UUID
///
/// # Request Body
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
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is not a valid UUID
/// - The amount or currency is invalid
/// - The account is not found
/// - The account has insufficient balance
/// - The account is closed or frozen
///
/// # Response
///
/// - `201 Created` - Withdrawal successful
/// - `400 Bad Request` - Invalid request data or insufficient balance
/// - `404 Not Found` - Account not found
/// - `409 Conflict` - Account is closed or frozen
///
/// # Example Response
///
/// ```json
/// {
///     "transaction_id": "...",
///     "amount": {
///         "amount": "3000",
///         "currency": "JPY"
///     },
///     "balance_after": {
///         "amount": "7000",
///         "currency": "JPY"
///     },
///     "timestamp": "2024-01-15T10:30:00Z"
/// }
/// ```
#[allow(clippy::unused_async)]
pub async fn withdraw(
    State(_dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
    Json(request): Json<WithdrawRequest>,
) -> Result<(StatusCode, Json<TransactionResponse>), ApiErrorResponse> {
    // Step 1: Validate and parse account ID (pure function)
    let account_id = AccountId::create(&account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let account_id = match account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Transform DTO to domain types (pure function)
    let amount = dto_to_money(&request.amount).map_left(|error| {
        let (status, api_error) = transformation_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let amount = match amount {
        lambars::control::Either::Right(amount) => amount,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 3: Create transaction ID from idempotency key (pure function)
    let transaction_id = TransactionId::from_idempotency_key(&request.idempotency_key);

    // Step 4: Execute workflow via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the account loaded

    // Mock: Return not found for demonstration
    let _ = (account_id, amount, transaction_id);
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({
                "account_id": account_id_string
            }),
        ),
    ))
}

/// POST /transfers - Transfer money between accounts.
///
/// Transfers the specified amount from one account to another.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Request Body
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
///
/// Note: The source account is identified from the request path in the full implementation.
/// For this endpoint, we expect it to be passed as a header or derived from authentication.
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account IDs are not valid UUIDs
/// - The amount or currency is invalid
/// - The source and destination accounts are the same
/// - The source or destination account is not found
/// - The source account has insufficient balance
/// - The source account is closed or frozen
///
/// # Response
///
/// - `201 Created` - Transfer successful
/// - `400 Bad Request` - Invalid request data or insufficient balance
/// - `404 Not Found` - Source or destination account not found
/// - `409 Conflict` - Source account is closed or frozen
///
/// # Example Response
///
/// ```json
/// {
///     "transfer_id": "...",
///     "from_account_id": "...",
///     "to_account_id": "...",
///     "amount": {
///         "amount": "2000",
///         "currency": "JPY"
///     },
///     "from_balance_after": {
///         "amount": "8000",
///         "currency": "JPY"
///     },
///     "timestamp": "2024-01-15T10:30:00Z"
/// }
/// ```
#[allow(clippy::unused_async)]
pub async fn transfer(
    State(_dependencies): State<AppDependencies>,
    Path(from_account_id_string): Path<String>,
    Json(request): Json<TransferRequest>,
) -> Result<(StatusCode, Json<TransferResponse>), ApiErrorResponse> {
    // Step 1: Validate and parse account IDs (pure functions)
    let from_account_id = AccountId::create(&from_account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let from_account_id = match from_account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    let to_account_id = AccountId::create(&request.to_account_id).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let to_account_id = match to_account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Transform DTO to domain types (pure function)
    let amount = dto_to_money(&request.amount).map_left(|error| {
        let (status, api_error) = transformation_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let amount = match amount {
        lambars::control::Either::Right(amount) => amount,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 3: Create transaction ID from idempotency key (pure function)
    let transaction_id = TransactionId::from_idempotency_key(&request.idempotency_key);

    // Step 4: Validate that source and destination are different
    if from_account_id == to_account_id {
        return Err(ApiErrorResponse::new(
            StatusCode::BAD_REQUEST,
            ApiError::with_details(
                "SAME_ACCOUNT_TRANSFER",
                "Cannot transfer to the same account",
                serde_json::json!({
                    "from_account_id": from_account_id.to_string(),
                    "to_account_id": to_account_id.to_string()
                }),
            ),
        ));
    }

    // Step 5: Execute workflow via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the accounts loaded

    // Mock: Return not found for demonstration
    let _ = (from_account_id, to_account_id, amount, transaction_id);
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The source account was not found",
            serde_json::json!({
                "account_id": from_account_id_string
            }),
        ),
    ))
}

/// GET /accounts/{id}/transactions - Get transaction history.
///
/// Retrieves the transaction history for an account with pagination.
///
/// # Path Parameters
///
/// - `id` - The account UUID
///
/// # Query Parameters
///
/// - `page` - Page number (default: 1)
/// - `page_size` - Items per page (default: 20)
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is not a valid UUID
/// - The account is not found
///
/// # Response
///
/// - `200 OK` - History retrieved
/// - `400 Bad Request` - Invalid account ID format
/// - `404 Not Found` - Account not found
///
/// # Example Response
///
/// ```json
/// {
///     "account_id": "...",
///     "transactions": [
///         {
///             "transaction_id": "...",
///             "transaction_type": "Deposit",
///             "amount": {...},
///             "balance_after": {...},
///             "counterparty_account_id": null,
///             "timestamp": "2024-01-15T10:30:00Z"
///         }
///     ],
///     "total": 100,
///     "page": 1,
///     "page_size": 20
/// }
/// ```
#[allow(clippy::unused_async)]
pub async fn get_transactions(
    State(_dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TransactionHistoryResponse>, ApiErrorResponse> {
    // Step 1: Validate and parse account ID (pure function)
    let account_id = AccountId::create(&account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let account_id = match account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Validate pagination parameters
    let page = if params.page == 0 { 1 } else { params.page };
    let page_size = params.page_size.clamp(1, 100);

    // Step 3: Query transaction history via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the account loaded

    // Mock: Return not found for demonstration
    let _ = (account_id, page, page_size);
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({
                "account_id": account_id_string
            }),
        ),
    ))
}

// =============================================================================
// Helper Functions (Pure)
// =============================================================================

/// Validates that the page number is positive.
///
/// Returns 1 if the input is 0 or negative.
#[allow(dead_code)]
const fn normalize_page(page: usize) -> usize {
    if page == 0 { 1 } else { page }
}

/// Clamps the page size to a reasonable range.
///
/// Minimum: 1, Maximum: 100
#[allow(dead_code)]
fn clamp_page_size(page_size: usize) -> usize {
    page_size.clamp(1, 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::dto::requests::MoneyDto;
    use rstest::rstest;

    // =========================================================================
    // normalize_page Tests
    // =========================================================================

    #[rstest]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(5, 5)]
    #[case(100, 100)]
    fn normalize_page_returns_expected(#[case] input: usize, #[case] expected: usize) {
        assert_eq!(normalize_page(input), expected);
    }

    // =========================================================================
    // clamp_page_size Tests
    // =========================================================================

    #[rstest]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(20, 20)]
    #[case(50, 50)]
    #[case(100, 100)]
    #[case(150, 100)]
    #[case(1000, 100)]
    fn clamp_page_size_returns_expected(#[case] input: usize, #[case] expected: usize) {
        assert_eq!(clamp_page_size(input), expected);
    }

    // =========================================================================
    // TransactionId from idempotency key Tests
    // =========================================================================

    #[rstest]
    fn transaction_id_from_idempotency_key_is_deterministic() {
        let key = "deposit-123-abc";

        let id1 = TransactionId::from_idempotency_key(key);
        let id2 = TransactionId::from_idempotency_key(key);

        assert_eq!(id1, id2);
    }

    #[rstest]
    fn transaction_id_from_different_keys_produces_different_ids() {
        let key1 = "deposit-123-abc";
        let key2 = "deposit-456-def";

        let id1 = TransactionId::from_idempotency_key(key1);
        let id2 = TransactionId::from_idempotency_key(key2);

        assert_ne!(id1, id2);
    }

    // =========================================================================
    // AccountId validation Tests
    // =========================================================================

    #[rstest]
    fn account_id_create_valid_uuid_returns_right() {
        let valid_uuid = "01234567-89ab-cdef-0123-456789abcdef";

        let result = AccountId::create(valid_uuid);

        assert!(result.is_right());
    }

    #[rstest]
    fn account_id_create_invalid_uuid_returns_left() {
        let invalid_uuid = "not-a-uuid";

        let result = AccountId::create(invalid_uuid);

        assert!(result.is_left());
    }

    // =========================================================================
    // dto_to_money Tests
    // =========================================================================

    #[rstest]
    fn dto_to_money_valid_returns_right() {
        let dto = MoneyDto {
            amount: "5000".to_string(),
            currency: "JPY".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_right());
    }

    #[rstest]
    fn dto_to_money_invalid_amount_returns_left() {
        let dto = MoneyDto {
            amount: "invalid".to_string(),
            currency: "JPY".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
    }

    #[rstest]
    fn dto_to_money_invalid_currency_returns_left() {
        let dto = MoneyDto {
            amount: "5000".to_string(),
            currency: "INVALID".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
    }

    // =========================================================================
    // PaginationParams Tests
    // =========================================================================

    #[rstest]
    fn pagination_params_default_values() {
        let params = PaginationParams::default();

        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }
}
