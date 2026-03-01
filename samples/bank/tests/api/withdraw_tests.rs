//! Integration tests for POST /accounts/{id}/withdraw endpoint.

use crate::common::*;
use reqwest::StatusCode;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn withdraw_success() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Withdraw Test User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Withdraw
    let withdraw_request = TransactionFactory::withdraw("3000", "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert!(!response.transaction_id.is_empty());
    assert_eq!(response.amount.amount, "3000");
    assert_eq!(response.amount.currency, "JPY");
    assert_eq!(response.balance_after.amount, "7000"); // 10000 - 3000
    assert_eq!(response.balance_after.currency, "JPY");
}

#[rstest]
#[tokio::test]
async fn withdraw_idempotency_returns_same_response() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Withdraw Idempotency Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Create request with fixed idempotency key
    let idempotency_key = "test-idempotency-withdraw-456";
    let withdraw_request = TransactionFactory::withdraw_with_key("3000", "JPY", idempotency_key);

    // First request - should succeed
    let result1 = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;
    assert_success(&result1);
    let response1 = result1.unwrap();

    // Second request with same idempotency key - should return same result
    let result2 = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;
    assert_success(&result2);
    let response2 = result2.unwrap();

    // Verify same transaction_id and balance
    assert_eq!(response1.transaction_id, response2.transaction_id);
    assert_eq!(response1.balance_after, response2.balance_after);

    // Verify balance is correct (not doubled withdrawal)
    let balance = client.get_balance(&created.account_id).await.unwrap();
    assert_eq!(balance.balance.amount, "7000"); // 10000 - 3000, not 4000
}

#[rstest]
#[tokio::test]
async fn withdraw_exact_balance() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Exact Balance Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Withdraw exact balance
    let withdraw_request = TransactionFactory::withdraw("10000", "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.balance_after.amount, "0");
}

#[rstest]
#[tokio::test]
async fn withdraw_insufficient_balance() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Insufficient Balance Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Try to withdraw more than balance
    let withdraw_request = TransactionFactory::withdraw("20000", "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_api_error(&result, "INSUFFICIENT_BALANCE", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn withdraw_account_not_found() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let non_existent_id = non_existent_uuid();

    let withdraw_request = TransactionFactory::withdraw("5000", "JPY");
    let result = client.withdraw(&non_existent_id, &withdraw_request).await;

    assert_api_error(&result, "ACCOUNT_NOT_FOUND", StatusCode::NOT_FOUND);
}

#[rstest]
#[case("not-a-valid-uuid")]
#[case("12345")]
#[tokio::test]
async fn withdraw_invalid_uuid(#[case] invalid_id: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    let withdraw_request = TransactionFactory::withdraw("5000", "JPY");
    let result = client.withdraw(invalid_id, &withdraw_request).await;

    assert_api_error(&result, "INVALID_ACCOUNT_ID", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("not-a-number")]
#[case("12.34.56")]
#[case("abc123")]
#[tokio::test]
async fn withdraw_invalid_amount(#[case] invalid_amount: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Invalid Amount Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    let withdraw_request = TransactionFactory::withdraw(invalid_amount, "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn withdraw_negative_amount() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Negative Amount Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    let withdraw_request = TransactionFactory::withdraw("-1000", "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn withdraw_zero_amount() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Zero Amount Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    let withdraw_request = TransactionFactory::withdraw("0", "JPY");
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("XYZ")]
#[case("BITCOIN")]
#[case("123")]
#[tokio::test]
async fn withdraw_invalid_currency(#[case] invalid_currency: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Invalid Currency Test");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    let withdraw_request = TransactionFactory::withdraw("5000", invalid_currency);
    let result = client
        .withdraw(&created.account_id, &withdraw_request)
        .await;

    assert_api_error(&result, "INVALID_CURRENCY", StatusCode::BAD_REQUEST);
}
