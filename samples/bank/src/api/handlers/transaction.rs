//! Transaction-related HTTP handlers.
//!
//! This module provides handlers for transaction operations:
//!
//! - `POST /accounts/{id}/deposit` - Deposit money
//! - `POST /accounts/{id}/withdraw` - Withdraw money
//! - `POST /accounts/{id}/transfer` - Transfer money between accounts
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
    MoneyResponseDto, TransactionHistoryResponse, TransactionResponse, TransferResponse,
};
use crate::api::dto::transformers::dto_to_money;
use crate::api::middleware::error_handler::{
    ApiError, ApiErrorResponse, account_id_error_to_api_error, domain_error_to_api_error,
    transformation_error_to_api_error,
};
use crate::application::queries::{GetHistoryQuery, build_transaction_history};
use crate::application::services::idempotency::{
    IdempotencyCheckResult, check_transaction_idempotency,
};
use crate::application::workflows::{FundingSourceType, deposit, transfer, withdraw};
use crate::domain::account::aggregate::Account;
use crate::domain::account::commands::{DepositCommand, TransferCommand, WithdrawCommand};
use crate::domain::account::events::AccountEvent;
use crate::domain::value_objects::{AccountId, Timestamp, TransactionId};
use crate::infrastructure::AppDependencies;

/// POST /accounts/{id}/deposit - Deposit money into an account.
///
/// Deposits the specified amount into the account.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is invalid
/// - The amount or currency is invalid
/// - The account is not found
/// - The account is closed
/// - Event store operation fails
#[allow(clippy::too_many_lines)]
pub async fn deposit_handler(
    State(dependencies): State<AppDependencies>,
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

    // Step 4: Load events and process in a single block to ensure PersistentList is dropped
    // before any subsequent await points (PersistentList is not Send)
    let account = match dependencies
        .event_store()
        .load_events(&account_id)
        .run_async()
        .await
    {
        Ok(events) => {
            // Step 4.5: Check idempotency (pure function)
            match check_transaction_idempotency(&events, &transaction_id) {
                IdempotencyCheckResult::AlreadyProcessed(AccountEvent::Deposited(existing)) => {
                    return Ok((
                        StatusCode::OK,
                        Json(TransactionResponse {
                            transaction_id: existing.transaction_id.to_string(),
                            amount: MoneyResponseDto {
                                amount: existing.amount.amount().to_string(),
                                currency: existing.amount.currency().to_string(),
                            },
                            balance_after: MoneyResponseDto {
                                amount: existing.balance_after.amount().to_string(),
                                currency: existing.balance_after.currency().to_string(),
                            },
                            timestamp: existing.deposited_at.to_iso_string(),
                        }),
                    ));
                }
                IdempotencyCheckResult::AlreadyProcessed(_) => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::CONFLICT,
                        ApiError::with_details(
                            "IDEMPOTENCY_CONFLICT",
                            "Transaction ID already used for a different operation type",
                            serde_json::json!({ "transaction_id": transaction_id.to_string() }),
                        ),
                    ));
                }
                IdempotencyCheckResult::NotFound => {}
            }

            // Step 4.6: Rebuild account from events (pure function)
            match Account::from_events(&events) {
                Some(account) => account,
                None => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::NOT_FOUND,
                        ApiError::with_details(
                            "ACCOUNT_NOT_FOUND",
                            "The specified account was not found",
                            serde_json::json!({ "account_id": account_id_string }),
                        ),
                    ));
                }
            }
        }
        Err(store_error) => {
            return Err(ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to load account events",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            ));
        }
    };

    // Step 5: Create domain command (pure function)
    let command = DepositCommand::new(account_id, amount, transaction_id);

    // Step 7: Execute workflow (pure function)
    let timestamp = Timestamp::now();
    let event_result = deposit(&command, &account, timestamp);

    let event = match event_result {
        lambars::control::Either::Right(event) => event,
        lambars::control::Either::Left(error) => {
            let (status, api_error) = domain_error_to_api_error(error);
            return Err(ApiErrorResponse::new(status, api_error));
        }
    };

    // Step 8: Persist event to event store (IO)
    let persist_result = dependencies
        .event_store()
        .append_events(
            &account_id,
            account.version,
            vec![AccountEvent::Deposited(event.clone())],
        )
        .run_async()
        .await;

    if let Err(store_error) = persist_result {
        return Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::with_details(
                "EVENT_STORE_ERROR",
                "Failed to persist deposit event",
                serde_json::json!({ "error": store_error.to_string() }),
            ),
        ));
    }

    // Step 9: Invalidate cache (IO)
    let _ = dependencies
        .read_model()
        .invalidate(&account_id)
        .run_async()
        .await;

    // Step 10: Transform result to response DTO (pure function)
    let response = TransactionResponse {
        transaction_id: event.transaction_id.to_string(),
        amount: MoneyResponseDto {
            amount: event.amount.amount().to_string(),
            currency: event.amount.currency().to_string(),
        },
        balance_after: MoneyResponseDto {
            amount: event.balance_after.amount().to_string(),
            currency: event.balance_after.currency().to_string(),
        },
        timestamp: event.deposited_at.to_iso_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /accounts/{id}/withdraw - Withdraw money from an account.
///
/// Withdraws the specified amount from the account.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is invalid
/// - The amount or currency is invalid
/// - The account is not found
/// - The account is closed or frozen
/// - Insufficient balance
/// - Event store operation fails
#[allow(clippy::too_many_lines)]
pub async fn withdraw_handler(
    State(dependencies): State<AppDependencies>,
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

    // Step 4: Load events and process in a single block to ensure PersistentList is dropped
    // before any subsequent await points (PersistentList is not Send)
    let account = match dependencies
        .event_store()
        .load_events(&account_id)
        .run_async()
        .await
    {
        Ok(events) => {
            // Step 4.5: Check idempotency (pure function)
            match check_transaction_idempotency(&events, &transaction_id) {
                IdempotencyCheckResult::AlreadyProcessed(AccountEvent::Withdrawn(existing)) => {
                    return Ok((
                        StatusCode::OK,
                        Json(TransactionResponse {
                            transaction_id: existing.transaction_id.to_string(),
                            amount: MoneyResponseDto {
                                amount: existing.amount.amount().to_string(),
                                currency: existing.amount.currency().to_string(),
                            },
                            balance_after: MoneyResponseDto {
                                amount: existing.balance_after.amount().to_string(),
                                currency: existing.balance_after.currency().to_string(),
                            },
                            timestamp: existing.withdrawn_at.to_iso_string(),
                        }),
                    ));
                }
                IdempotencyCheckResult::AlreadyProcessed(_) => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::CONFLICT,
                        ApiError::with_details(
                            "IDEMPOTENCY_CONFLICT",
                            "Transaction ID already used for a different operation type",
                            serde_json::json!({ "transaction_id": transaction_id.to_string() }),
                        ),
                    ));
                }
                IdempotencyCheckResult::NotFound => {}
            }

            // Step 4.6: Rebuild account from events (pure function)
            match Account::from_events(&events) {
                Some(account) => account,
                None => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::NOT_FOUND,
                        ApiError::with_details(
                            "ACCOUNT_NOT_FOUND",
                            "The specified account was not found",
                            serde_json::json!({ "account_id": account_id_string }),
                        ),
                    ));
                }
            }
        }
        Err(store_error) => {
            return Err(ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to load account events",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            ));
        }
    };

    // Step 5: Create domain command (pure function)
    let command = WithdrawCommand::new(account_id, amount, transaction_id);

    // Step 6: Define funding sources (configuration)
    let funding_sources = vec![FundingSourceType::Balance];

    // Step 8: Execute workflow (pure function)
    let timestamp = Timestamp::now();
    let event_result = withdraw(&command, &account, &funding_sources, timestamp);

    let event = match event_result {
        lambars::control::Either::Right(event) => event,
        lambars::control::Either::Left(error) => {
            let (status, api_error) = domain_error_to_api_error(error);
            return Err(ApiErrorResponse::new(status, api_error));
        }
    };

    // Step 9: Persist event to event store (IO)
    let persist_result = dependencies
        .event_store()
        .append_events(
            &account_id,
            account.version,
            vec![AccountEvent::Withdrawn(event.clone())],
        )
        .run_async()
        .await;

    if let Err(store_error) = persist_result {
        return Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::with_details(
                "EVENT_STORE_ERROR",
                "Failed to persist withdrawal event",
                serde_json::json!({ "error": store_error.to_string() }),
            ),
        ));
    }

    // Step 10: Invalidate cache (IO)
    let _ = dependencies
        .read_model()
        .invalidate(&account_id)
        .run_async()
        .await;

    // Step 11: Transform result to response DTO (pure function)
    let response = TransactionResponse {
        transaction_id: event.transaction_id.to_string(),
        amount: MoneyResponseDto {
            amount: event.amount.amount().to_string(),
            currency: event.amount.currency().to_string(),
        },
        balance_after: MoneyResponseDto {
            amount: event.balance_after.amount().to_string(),
            currency: event.balance_after.currency().to_string(),
        },
        timestamp: event.withdrawn_at.to_iso_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /accounts/{id}/transfer - Transfer money between accounts.
///
/// Transfers the specified amount from one account to another.
/// The idempotency key ensures duplicate requests are handled safely.
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - Either account ID is invalid
/// - The amount or currency is invalid
/// - Either account is not found
/// - The source account is closed or frozen
/// - The destination account is closed
/// - Transferring to the same account
/// - Insufficient balance
/// - Event store operation fails
#[allow(clippy::too_many_lines)]
pub async fn transfer_handler(
    State(dependencies): State<AppDependencies>,
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

    // Step 3: Validate that source and destination are different
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

    // Step 4: Create transaction ID from idempotency key (pure function)
    let transaction_id = TransactionId::from_idempotency_key(&request.idempotency_key);

    // Step 5: Load source account events and process in a single block to ensure PersistentList
    // is dropped before any subsequent await points (PersistentList is not Send)
    let from_account = match dependencies
        .event_store()
        .load_events(&from_account_id)
        .run_async()
        .await
    {
        Ok(events) => {
            // Step 5.5: Check idempotency on source account (pure function)
            match check_transaction_idempotency(&events, &transaction_id) {
                IdempotencyCheckResult::AlreadyProcessed(AccountEvent::TransferSent(existing)) => {
                    return Ok((
                        StatusCode::OK,
                        Json(TransferResponse {
                            transfer_id: existing.transaction_id.to_string(),
                            from_account_id: existing.account_id.to_string(),
                            to_account_id: existing.to_account_id.to_string(),
                            amount: MoneyResponseDto {
                                amount: existing.amount.amount().to_string(),
                                currency: existing.amount.currency().to_string(),
                            },
                            from_balance_after: MoneyResponseDto {
                                amount: existing.balance_after.amount().to_string(),
                                currency: existing.balance_after.currency().to_string(),
                            },
                            timestamp: existing.sent_at.to_iso_string(),
                        }),
                    ));
                }
                IdempotencyCheckResult::AlreadyProcessed(_) => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::CONFLICT,
                        ApiError::with_details(
                            "IDEMPOTENCY_CONFLICT",
                            "Transaction ID already used for a different operation type",
                            serde_json::json!({ "transaction_id": transaction_id.to_string() }),
                        ),
                    ));
                }
                IdempotencyCheckResult::NotFound => {}
            }

            // Step 5.6: Rebuild source account from events (pure function)
            match Account::from_events(&events) {
                Some(account) => account,
                None => {
                    return Err(ApiErrorResponse::new(
                        StatusCode::NOT_FOUND,
                        ApiError::with_details(
                            "ACCOUNT_NOT_FOUND",
                            "The source account was not found",
                            serde_json::json!({ "account_id": from_account_id_string }),
                        ),
                    ));
                }
            }
        }
        Err(store_error) => {
            return Err(ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to load source account events",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            ));
        }
    };

    // Step 6: Load destination account from event store and rebuild (IO + pure)
    let to_account = match dependencies
        .event_store()
        .load_events(&to_account_id)
        .run_async()
        .await
    {
        Ok(events) => match Account::from_events(&events) {
            Some(account) => account,
            None => {
                return Err(ApiErrorResponse::new(
                    StatusCode::NOT_FOUND,
                    ApiError::with_details(
                        "ACCOUNT_NOT_FOUND",
                        "The destination account was not found",
                        serde_json::json!({ "account_id": request.to_account_id }),
                    ),
                ));
            }
        },
        Err(store_error) => {
            return Err(ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to load destination account events",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            ));
        }
    };

    // Step 7: Create domain command (pure function)
    let command = TransferCommand::new(from_account_id, to_account_id, amount, transaction_id);

    // Step 8: Execute workflow (pure function)
    let timestamp = Timestamp::now();
    let events_result = transfer(&command, &from_account, &to_account, timestamp);

    let (sent_event, received_event) = match events_result {
        lambars::control::Either::Right(events) => events,
        lambars::control::Either::Left(error) => {
            let (status, api_error) = domain_error_to_api_error(error);
            return Err(ApiErrorResponse::new(status, api_error));
        }
    };

    // Step 9: Persist events to event store (IO)
    // Note: In a real implementation, this should be transactional
    let from_persist_result = dependencies
        .event_store()
        .append_events(
            &from_account_id,
            from_account.version,
            vec![AccountEvent::TransferSent(sent_event.clone())],
        )
        .run_async()
        .await;

    if let Err(store_error) = from_persist_result {
        return Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::with_details(
                "EVENT_STORE_ERROR",
                "Failed to persist transfer sent event",
                serde_json::json!({ "error": store_error.to_string() }),
            ),
        ));
    }

    let to_persist_result = dependencies
        .event_store()
        .append_events(
            &to_account_id,
            to_account.version,
            vec![AccountEvent::TransferReceived(received_event.clone())],
        )
        .run_async()
        .await;

    if let Err(store_error) = to_persist_result {
        return Err(ApiErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::with_details(
                "EVENT_STORE_ERROR",
                "Failed to persist transfer received event",
                serde_json::json!({ "error": store_error.to_string() }),
            ),
        ));
    }

    // Step 10: Invalidate cache for both accounts (IO)
    let _ = dependencies
        .read_model()
        .invalidate(&from_account_id)
        .run_async()
        .await;
    let _ = dependencies
        .read_model()
        .invalidate(&to_account_id)
        .run_async()
        .await;

    // Step 11: Transform result to response DTO (pure function)
    let response = TransferResponse {
        transfer_id: sent_event.transaction_id.to_string(),
        from_account_id: sent_event.account_id.to_string(),
        to_account_id: sent_event.to_account_id.to_string(),
        amount: MoneyResponseDto {
            amount: sent_event.amount.amount().to_string(),
            currency: sent_event.amount.currency().to_string(),
        },
        from_balance_after: MoneyResponseDto {
            amount: sent_event.balance_after.amount().to_string(),
            currency: sent_event.balance_after.currency().to_string(),
        },
        timestamp: sent_event.sent_at.to_iso_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /accounts/{id}/transactions - Get transaction history.
///
/// Retrieves the transaction history for an account with pagination.
///
/// # Errors
///
/// Returns `ApiErrorResponse` if:
/// - The account ID is invalid
/// - The account is not found
/// - Event store operation fails
pub async fn get_transactions(
    State(dependencies): State<AppDependencies>,
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

    // Step 3: Load events from event store (IO)
    let events_result = dependencies
        .event_store()
        .load_events(&account_id)
        .run_async()
        .await;

    let events = match events_result {
        Ok(events) => events,
        Err(store_error) => {
            return Err(ApiErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::with_details(
                    "EVENT_STORE_ERROR",
                    "Failed to load account events",
                    serde_json::json!({ "error": store_error.to_string() }),
                ),
            ));
        }
    };

    // Step 4: Check if account exists
    if Account::from_events(&events).is_none() {
        return Err(ApiErrorResponse::new(
            StatusCode::NOT_FOUND,
            ApiError::with_details(
                "ACCOUNT_NOT_FOUND",
                "The specified account was not found",
                serde_json::json!({ "account_id": account_id_string }),
            ),
        ));
    }

    // Step 5: Build transaction history (pure function)
    // Convert page/page_size to offset/limit for the query
    let offset = (page - 1) * page_size;
    let limit = page_size;
    let query = GetHistoryQuery::new(account_id, offset, limit);

    let history = build_transaction_history(account_id, &events, &query);

    // Step 6: Transform to response DTO (pure function)
    let transactions: Vec<_> = history
        .transactions
        .iter()
        .map(|record| crate::api::dto::responses::TransactionRecordDto {
            transaction_id: record.transaction_id.to_string(),
            transaction_type: format!("{:?}", record.transaction_type),
            amount: MoneyResponseDto {
                amount: record.amount.amount().to_string(),
                currency: record.amount.currency().to_string(),
            },
            balance_after: MoneyResponseDto {
                amount: record.balance_after.amount().to_string(),
                currency: record.balance_after.currency().to_string(),
            },
            counterparty_account_id: record.counterparty.map(|id| id.to_string()),
            timestamp: record.timestamp.to_iso_string(),
        })
        .collect();

    Ok(Json(TransactionHistoryResponse {
        account_id: account_id.to_string(),
        transactions,
        total: history.total,
        page,
        page_size,
    }))
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
