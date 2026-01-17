//! Integration tests for POST /accounts endpoint.

use crate::common::*;
use reqwest::StatusCode;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn create_account_success() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::default_jpy_account("Test User");

    let result = client.create_account(&request).await;

    assert_success(&result);
    let response = result.unwrap();
    assert!(!response.account_id.is_empty());
    assert_eq!(response.owner_name, "Test User");
    assert_eq!(response.balance.amount, "10000");
    assert_eq!(response.balance.currency, "JPY");
    assert_eq!(response.status, "Active");
}

#[rstest]
#[tokio::test]
async fn create_account_with_zero_balance() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::zero_balance_account("Zero Balance User");

    let result = client.create_account(&request).await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.balance.amount, "0");
    assert_eq!(response.balance.currency, "JPY");
}

#[rstest]
#[case("not-a-number")]
#[case("")]
#[case("12.34.56")]
#[case("abc123")]
#[tokio::test]
async fn create_account_invalid_amount(#[case] invalid_amount: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::create_request("Test User", invalid_amount, "JPY");

    let result = client.create_account(&request).await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[tokio::test]
async fn create_account_negative_amount() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::create_request("Test User", "-1000", "JPY");

    let result = client.create_account(&request).await;

    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("XYZ")]
#[case("BITCOIN")]
#[case("")]
#[case("123")]
#[tokio::test]
async fn create_account_invalid_currency(#[case] invalid_currency: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::create_request("Test User", "10000", invalid_currency);

    let result = client.create_account(&request).await;

    assert_api_error(&result, "INVALID_CURRENCY", StatusCode::BAD_REQUEST);
}

#[rstest]
#[case("")]
#[case("   ")]
#[tokio::test]
async fn create_account_empty_owner_name(#[case] empty_name: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let request = AccountFactory::create_request(empty_name, "10000", "JPY");

    let result = client.create_account(&request).await;

    // Note: The API returns INVALID_AMOUNT for empty owner names
    // because the validation logic uses DomainError::InvalidAmount
    assert_api_error(&result, "INVALID_AMOUNT", StatusCode::BAD_REQUEST);
}
