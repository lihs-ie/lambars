//! Integration tests for POST /accounts/{id}/transfer endpoint.

use crate::common::*;
use reqwest::StatusCode;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn transfer_success() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer From User");
    let to_request = AccountFactory::default_jpy_account("Transfer To User");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    // Transfer
    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "3000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert!(!response.transfer_id.is_empty());
    assert_eq!(response.from_account_id, from_account.account_id);
    assert_eq!(response.to_account_id, to_account.account_id);
    assert_eq!(response.amount.amount, "3000");
    assert_eq!(response.amount.currency, "JPY");
    assert_eq!(response.from_balance_after.amount, "7000"); // 10000 - 3000

    // Verify destination balance
    let to_balance = client.get_balance(&to_account.account_id).await.unwrap();
    assert_eq!(to_balance.balance.amount, "13000"); // 10000 + 3000
}

#[rstest]
#[tokio::test]
async fn transfer_idempotency_returns_same_response() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Idempotency From");
    let to_request = AccountFactory::default_jpy_account("Transfer Idempotency To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    // Create request with fixed idempotency key
    let idempotency_key = "test-idempotency-transfer-789";
    let transfer_request = TransactionFactory::transfer_with_key(
        &to_account.account_id,
        "3000",
        "JPY",
        idempotency_key,
    );

    // First request - should succeed
    let result1 = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;
    assert_success(&result1);
    let response1 = result1.unwrap();

    // Second request with same idempotency key - should return same result
    let result2 = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;
    assert_success(&result2);
    let response2 = result2.unwrap();

    // Verify same transfer_id and balance
    assert_eq!(response1.transfer_id, response2.transfer_id);
    assert_eq!(response1.from_balance_after, response2.from_balance_after);

    // Verify balances are correct (not doubled transfer)
    let from_balance = client.get_balance(&from_account.account_id).await.unwrap();
    let to_balance = client.get_balance(&to_account.account_id).await.unwrap();
    assert_eq!(from_balance.balance.amount, "7000"); // 10000 - 3000, not 4000
    assert_eq!(to_balance.balance.amount, "13000"); // 10000 + 3000, not 16000
}

#[rstest]
#[tokio::test]
async fn transfer_exact_balance() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Exact From");
    let to_request = AccountFactory::default_jpy_account("Transfer Exact To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    // Transfer exact balance
    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "10000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.from_balance_after.amount, "0");
}

#[rstest]
#[tokio::test]
async fn transfer_insufficient_balance() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Insufficient From");
    let to_request = AccountFactory::default_jpy_account("Transfer Insufficient To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    // Try to transfer more than balance
    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "20000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INSUFFICIENT_BALANCE", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn transfer_same_account() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account
    let request = AccountFactory::default_jpy_account("Self Transfer User");
    let result = client.create_account(&request).await;
    assert_success(&result);
    let account = result.unwrap();

    // Try to transfer to self
    let transfer_request = TransactionFactory::transfer(&account.account_id, "5000", "JPY");
    let result = client
        .transfer(&account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "SAME_ACCOUNT_TRANSFER", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn transfer_source_not_found() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create destination account
    let to_request = AccountFactory::default_jpy_account("Transfer To User");
    let to_result = client.create_account(&to_request).await;
    assert_success(&to_result);
    let to_account = to_result.unwrap();

    // Try to transfer from non-existent account
    let non_existent_id = non_existent_uuid();
    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "5000", "JPY");
    let result = client.transfer(&non_existent_id, &transfer_request).await;

    assert_api_error(&result, "ACCOUNT_NOT_FOUND", StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn transfer_destination_not_found() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create source account
    let from_request = AccountFactory::default_jpy_account("Transfer From User");
    let from_result = client.create_account(&from_request).await;
    assert_success(&from_result);
    let from_account = from_result.unwrap();

    // Try to transfer to non-existent account
    let non_existent_id = non_existent_uuid();
    let transfer_request = TransactionFactory::transfer(&non_existent_id, "5000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "ACCOUNT_NOT_FOUND", StatusCode::NOT_FOUND);
}

#[rstest]
#[case("not-a-valid-uuid")]
#[case("12345")]
#[tokio::test]
async fn transfer_invalid_source_uuid(#[case] invalid_id: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create destination account
    let to_request = AccountFactory::default_jpy_account("Transfer To User");
    let to_result = client.create_account(&to_request).await;
    assert_success(&to_result);
    let to_account = to_result.unwrap();

    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "5000", "JPY");
    let result = client.transfer(invalid_id, &transfer_request).await;

    assert_api_error(&result, "INVALID_ACCOUNT_ID", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("not-a-valid-uuid")]
#[case("12345")]
#[tokio::test]
async fn transfer_invalid_destination_uuid(#[case] invalid_id: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create source account
    let from_request = AccountFactory::default_jpy_account("Transfer From User");
    let from_result = client.create_account(&from_request).await;
    assert_success(&from_result);
    let from_account = from_result.unwrap();

    let transfer_request = TransactionFactory::transfer(invalid_id, "5000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INVALID_ACCOUNT_ID", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("not-a-number")]
#[case("12.34.56")]
#[case("abc123")]
#[tokio::test]
async fn transfer_invalid_amount(#[case] invalid_amount: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Invalid Amount From");
    let to_request = AccountFactory::default_jpy_account("Transfer Invalid Amount To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    let transfer_request =
        TransactionFactory::transfer(&to_account.account_id, invalid_amount, "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn transfer_negative_amount() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Negative From");
    let to_request = AccountFactory::default_jpy_account("Transfer Negative To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "-1000", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn transfer_zero_amount() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Zero From");
    let to_request = AccountFactory::default_jpy_account("Transfer Zero To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "0", "JPY");
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("XYZ")]
#[case("BITCOIN")]
#[case("123")]
#[tokio::test]
async fn transfer_invalid_currency(#[case] invalid_currency: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("Transfer Invalid Currency From");
    let to_request = AccountFactory::default_jpy_account("Transfer Invalid Currency To");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    let transfer_request =
        TransactionFactory::transfer(&to_account.account_id, "5000", invalid_currency);
    let result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;

    assert_api_error(&result, "INVALID_CURRENCY", StatusCode::BAD_REQUEST);
}
