//! Integration tests for GET /accounts/{id}/balance endpoint.

use crate::common::*;
use reqwest::StatusCode;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn get_balance_success() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account first
    let create_request = AccountFactory::default_jpy_account("Balance Test User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Get balance
    let result = client.get_balance(&created.account_id).await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.account_id, created.account_id);
    assert_eq!(response.balance.amount, "10000");
    assert_eq!(response.balance.currency, "JPY");
}

#[rstest]
#[tokio::test]
async fn get_balance_not_found() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let non_existent_id = non_existent_uuid();

    let result = client.get_balance(&non_existent_id).await;

    assert_api_error(&result, "ACCOUNT_NOT_FOUND", StatusCode::NOT_FOUND);
}

#[rstest]
#[case("not-a-valid-uuid")]
#[case("12345")]
#[case("")]
#[tokio::test]
async fn get_balance_invalid_uuid(#[case] invalid_id: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    let result = client.get_balance(invalid_id).await;

    assert_api_error(&result, "INVALID_ACCOUNT_ID", StatusCode::BAD_REQUEST);
}
