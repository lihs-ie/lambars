//! Account-related HTTP handlers.
//!
//! This module provides handlers for account operations:
//!
//! - `POST /accounts` - Create a new account
//! - `GET /accounts/{id}` - Get account information
//! - `GET /accounts/{id}/balance` - Get account balance
//!
//! # Functional Design
//!
//! Handlers follow a pipeline pattern:
//!
//! ```text
//! Request → Validate → Transform → Execute → Transform → Response
//! ```
//!
//! Each step is a pure function except for the Execute step which
//! runs the `AsyncIO` computation.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::api::dto::requests::OpenAccountRequest;
use crate::api::dto::responses::{AccountResponse, BalanceResponse, MoneyResponseDto};
use crate::api::dto::transformers::{account_to_response, dto_to_money, money_to_dto};
use crate::api::middleware::error_handler::{
    ApiError, ApiErrorResponse, account_id_error_to_api_error, transformation_error_to_api_error,
};
use crate::domain::account::commands::OpenAccountCommand;
use crate::domain::value_objects::AccountId;
use crate::infrastructure::AppDependencies;

/// POST /accounts - Create a new account.
///
/// Creates a new bank account with the specified owner name and initial balance.
///
/// # Request Body
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
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The initial balance amount or currency is invalid
///
/// # Response
///
/// - `201 Created` - Account created successfully
/// - `400 Bad Request` - Invalid request data
///
/// # Example Response
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
#[allow(clippy::unused_async)]
pub async fn create_account(
    State(_dependencies): State<AppDependencies>,
    Json(request): Json<OpenAccountRequest>,
) -> Result<(StatusCode, Json<AccountResponse>), ApiErrorResponse> {
    // Step 1: Transform DTO to domain types (pure function)
    let initial_balance = dto_to_money(&request.initial_balance).map_left(|error| {
        let (status, api_error) = transformation_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let initial_balance = match initial_balance {
        lambars::control::Either::Right(balance) => balance,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Create domain command (pure function)
    let _command = OpenAccountCommand::new(request.owner_name.clone(), initial_balance.clone());

    // Step 3: Execute workflow via AsyncIO (would be implemented with dependencies)
    // For now, create a mock response
    let account_id = AccountId::generate();

    // Step 4: Transform result to response DTO (pure function)
    let response = AccountResponse {
        account_id: account_id.to_string(),
        owner_name: request.owner_name,
        balance: money_to_dto(&initial_balance),
        status: "Active".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /accounts/{id} - Get account information.
///
/// Retrieves the full account information including balance and status.
///
/// # Path Parameters
///
/// - `id` - The account UUID
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is not a valid UUID
/// - The account is not found
///
/// # Response
///
/// - `200 OK` - Account found
/// - `400 Bad Request` - Invalid account ID format
/// - `404 Not Found` - Account not found
///
/// # Example Response
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
#[allow(clippy::unused_async)]
pub async fn get_account(
    State(_dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
) -> Result<Json<AccountResponse>, ApiErrorResponse> {
    // Step 1: Validate and parse account ID (pure function)
    let account_id = AccountId::create(&account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let account_id = match account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Query account via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the account loaded
    // In a real implementation, this would query the event store and rebuild the aggregate

    // Mock: Return not found for demonstration
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({
                "account_id": account_id.to_string()
            }),
        ),
    ))
}

/// GET /accounts/{id}/balance - Get account balance.
///
/// Retrieves only the current balance of the account.
/// This is a lighter-weight query than getting the full account.
///
/// # Path Parameters
///
/// - `id` - The account UUID
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is not a valid UUID
/// - The account is not found
///
/// # Response
///
/// - `200 OK` - Balance retrieved
/// - `400 Bad Request` - Invalid account ID format
/// - `404 Not Found` - Account not found
///
/// # Example Response
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
#[allow(clippy::unused_async)]
pub async fn get_balance(
    State(_dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
) -> Result<Json<BalanceResponse>, ApiErrorResponse> {
    // Step 1: Validate and parse account ID (pure function)
    let account_id = AccountId::create(&account_id_string).map_left(|error| {
        let (status, api_error) = account_id_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    });

    let account_id = match account_id {
        lambars::control::Either::Right(id) => id,
        lambars::control::Either::Left(error) => return Err(error),
    };

    // Step 2: Query balance via AsyncIO (would be implemented with dependencies)
    // For now, return a not found error as we don't have the account loaded

    // Mock: Return not found for demonstration
    Err(ApiErrorResponse::new(
        StatusCode::NOT_FOUND,
        ApiError::with_details(
            "ACCOUNT_NOT_FOUND",
            "The specified account was not found",
            serde_json::json!({
                "account_id": account_id.to_string()
            }),
        ),
    ))
}

// =============================================================================
// Helper Functions (Pure)
// =============================================================================

/// Converts an account aggregate to the API response format.
///
/// This is a pure function that delegates to the transformer.
#[allow(dead_code)]
fn to_account_response(account: &crate::domain::account::aggregate::Account) -> AccountResponse {
    account_to_response(account)
}

/// Creates a balance response from account ID and balance.
///
/// This is a pure function.
#[allow(dead_code)]
fn to_balance_response(
    account_id: AccountId,
    balance: &crate::domain::value_objects::Money,
) -> BalanceResponse {
    BalanceResponse {
        account_id: account_id.to_string(),
        balance: MoneyResponseDto {
            amount: balance.amount().to_string(),
            currency: balance.currency().to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::dto::requests::MoneyDto;
    use crate::domain::account::aggregate::{Account, AccountStatus};
    use crate::domain::value_objects::{Currency, Money};
    use rstest::rstest;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    fn create_test_account() -> Account {
        Account {
            id: AccountId::generate(),
            owner_name: "Test User".to_string(),
            balance: Money::new(10000, Currency::JPY),
            status: AccountStatus::Active,
            version: 1,
        }
    }

    // =========================================================================
    // to_account_response Tests
    // =========================================================================

    #[rstest]
    fn to_account_response_transforms_correctly() {
        let account = create_test_account();

        let response = to_account_response(&account);

        assert_eq!(response.account_id, account.id.to_string());
        assert_eq!(response.owner_name, account.owner_name);
        assert_eq!(
            response.balance.amount,
            account.balance.amount().to_string()
        );
        assert_eq!(response.status, "Active");
    }

    #[rstest]
    fn to_account_response_is_pure() {
        let account = create_test_account();

        let response1 = to_account_response(&account);
        let response2 = to_account_response(&account);

        assert_eq!(response1, response2);
    }

    // =========================================================================
    // to_balance_response Tests
    // =========================================================================

    #[rstest]
    fn to_balance_response_transforms_correctly() {
        let account_id = AccountId::generate();
        let balance = Money::new(10000, Currency::JPY);

        let response = to_balance_response(account_id, &balance);

        assert_eq!(response.account_id, account_id.to_string());
        assert_eq!(response.balance.amount, "10000");
        assert_eq!(response.balance.currency, "JPY");
    }

    #[rstest]
    fn to_balance_response_is_pure() {
        let account_id = AccountId::generate();
        let balance = Money::new(5000, Currency::USD);

        let response1 = to_balance_response(account_id, &balance);
        let response2 = to_balance_response(account_id, &balance);

        assert_eq!(response1, response2);
    }

    // =========================================================================
    // Request DTO Transformation Tests
    // =========================================================================

    #[rstest]
    fn dto_to_money_transforms_valid_request() {
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
    fn dto_to_money_returns_error_for_invalid_amount() {
        let dto = MoneyDto {
            amount: "not-a-number".to_string(),
            currency: "JPY".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
    }

    #[rstest]
    fn dto_to_money_returns_error_for_invalid_currency() {
        let dto = MoneyDto {
            amount: "10000".to_string(),
            currency: "INVALID".to_string(),
        };

        let result = dto_to_money(&dto);

        assert!(result.is_left());
    }
}
