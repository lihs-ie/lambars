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
use crate::api::dto::transformers::account_to_response;
use crate::api::handlers::pipeline::{
    account_not_found_response, event_store_error_response, parse_account_id_for_api,
    parse_money_for_api,
};
use crate::api::middleware::error_handler::{
    ApiError, ApiErrorResponse, domain_error_to_api_error,
};
use crate::application::workflows::open_account;
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::OpenAccountCommand;
use crate::domain::account::events::AccountEvent;
use crate::domain::value_objects::{AccountId, Timestamp};
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
pub async fn create_account(
    State(dependencies): State<AppDependencies>,
    Json(request): Json<OpenAccountRequest>,
) -> Result<(StatusCode, Json<AccountResponse>), ApiErrorResponse> {
    // Step 1: Transform DTO to domain types using pipeline (pure function)
    let initial_balance = parse_money_for_api(&request.initial_balance)?;

    // Step 2: Create domain command (pure function)
    let command = OpenAccountCommand::new(request.owner_name.clone(), initial_balance.clone());

    // Step 3: Generate IDs and timestamp (these are the IO boundaries)
    let account_id = AccountId::generate();
    let timestamp = Timestamp::now();

    // Step 4: Execute workflow and convert Either to Result for pipeline compatibility
    let event = crate::api::handlers::pipeline::either_to_result(open_account(
        &command, account_id, timestamp,
    ))
    .map_err(|error| {
        let (status, api_error) = domain_error_to_api_error(error);
        ApiErrorResponse::new(status, api_error)
    })?;

    // Step 5: Persist event to event store (IO)
    dependencies
        .event_store()
        .append_events(&account_id, 0, vec![AccountEvent::Opened(event.clone())])
        .run_async()
        .await
        .map_err(|store_error| {
            ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to persist account creation event",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            )
        })?;

    // Step 6: Transform result to response DTO (pure function)
    let response = AccountResponse {
        account_id: event.account_id.to_string(),
        owner_name: event.owner_name,
        balance: MoneyResponseDto {
            amount: event.initial_balance.amount().to_string(),
            currency: event.initial_balance.currency().to_string(),
        },
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
pub async fn get_account(
    State(dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
) -> Result<Json<AccountResponse>, ApiErrorResponse> {
    // Step 1: Validate and parse account ID using pipeline (pure function)
    let account_id = parse_account_id_for_api(&account_id_string)?;

    // Step 2: Load events from event store (IO)
    let events = dependencies
        .event_store()
        .load_events(&account_id)
        .run_async()
        .await
        .map_err(|e| event_store_error_response(&e))?;

    // Step 3: Rebuild account from events (pure function)
    let account = Account::from_events(&events);

    // Step 4: Transform to response DTO (pure function)
    account.map_or_else(
        || Err(account_not_found_response(&account_id.to_string())),
        |acc| Ok(Json(account_to_response(&acc))),
    )
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
pub async fn get_balance(
    State(dependencies): State<AppDependencies>,
    Path(account_id_string): Path<String>,
) -> Result<Json<BalanceResponse>, ApiErrorResponse> {
    // Step 1: Validate and parse account ID using pipeline (pure function)
    let account_id = parse_account_id_for_api(&account_id_string)?;

    // Step 2: Try to get from cache first (IO)
    let cached_result = dependencies
        .read_model()
        .get_balance(&account_id)
        .run_async()
        .await;

    if let Ok(Some(cached)) = cached_result {
        // Cache hit - return cached balance
        return Ok(Json(BalanceResponse {
            account_id: account_id.to_string(),
            balance: MoneyResponseDto {
                amount: cached.balance.amount().to_string(),
                currency: cached.balance.currency().to_string(),
            },
        }));
    }

    // Step 3: Cache miss - load events from event store and rebuild account (IO + pure)
    // Note: We immediately extract the Account from events to ensure the non-Send
    // PersistentList is dropped before any subsequent await points.
    let account = dependencies
        .event_store()
        .load_events(&account_id)
        .run_async()
        .await
        .map_err(|e| event_store_error_response(&e))
        .map(|events| Account::from_events(&events))?;

    match account {
        Some(account) => {
            // Step 4: Update cache (IO - fire and forget)
            let _ = dependencies
                .read_model()
                .set_balance(&account_id, &account.balance, account.version)
                .run_async()
                .await;

            // Step 5: Transform to response DTO (pure function)
            Ok(Json(BalanceResponse {
                account_id: account_id.to_string(),
                balance: MoneyResponseDto {
                    amount: account.balance.amount().to_string(),
                    currency: account.balance.currency().to_string(),
                },
            }))
        }
        None => Err(account_not_found_response(&account_id.to_string())),
    }
}

// =============================================================================
// Helper Functions (Pure)
// =============================================================================

/// Converts an account aggregate to the API response format.
///
/// This is a pure function that delegates to the transformer.
#[allow(dead_code)]
fn to_account_response(account: &Account) -> AccountResponse {
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
    use crate::api::dto::transformers::dto_to_money;
    use crate::domain::account::aggregate::AccountStatus;
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
