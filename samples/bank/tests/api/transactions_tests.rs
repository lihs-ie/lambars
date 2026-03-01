//! Integration tests for GET /accounts/{id}/transactions endpoint.

use crate::common::*;
use reqwest::StatusCode;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn get_transactions_empty() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account (no transactions yet except initial deposit implicit in creation)
    let create_request = AccountFactory::default_jpy_account("Empty Transactions User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    let result = client
        .get_transactions(&created.account_id, None, None)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.account_id, created.account_id);
    // Account creation doesn't create a transaction, so it should be empty
    assert!(response.transactions.is_empty());
    assert_eq!(response.total, 0);
}

#[rstest]
#[tokio::test]
async fn get_transactions_with_deposits() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account and make deposits
    let create_request = AccountFactory::default_jpy_account("Deposit Transactions User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Make 3 deposits
    for _ in 0..3 {
        let deposit_request = TransactionFactory::deposit("1000", "JPY");
        let deposit_result = client.deposit(&created.account_id, &deposit_request).await;
        assert_success(&deposit_result);
    }

    let result = client
        .get_transactions(&created.account_id, None, None)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.transactions.len(), 3);
    assert_eq!(response.total, 3);
    for tx in &response.transactions {
        assert_eq!(tx.transaction_type, "Deposit");
        assert_eq!(tx.amount.amount, "1000");
    }
}

#[rstest]
#[tokio::test]
async fn get_transactions_with_all_types() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create two accounts
    let from_request = AccountFactory::default_jpy_account("All Types From User");
    let to_request = AccountFactory::default_jpy_account("All Types To User");
    let from_result = client.create_account(&from_request).await;
    let to_result = client.create_account(&to_request).await;
    assert_success(&from_result);
    assert_success(&to_result);
    let from_account = from_result.unwrap();
    let to_account = to_result.unwrap();

    // Deposit
    let deposit_request = TransactionFactory::deposit("5000", "JPY");
    let deposit_result = client
        .deposit(&from_account.account_id, &deposit_request)
        .await;
    assert_success(&deposit_result);

    // Withdraw
    let withdraw_request = TransactionFactory::withdraw("2000", "JPY");
    let withdraw_result = client
        .withdraw(&from_account.account_id, &withdraw_request)
        .await;
    assert_success(&withdraw_result);

    // Transfer
    let transfer_request = TransactionFactory::transfer(&to_account.account_id, "1000", "JPY");
    let transfer_result = client
        .transfer(&from_account.account_id, &transfer_request)
        .await;
    assert_success(&transfer_result);

    // Get transactions for from_account
    let result = client
        .get_transactions(&from_account.account_id, None, None)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.total, 3);

    // Find each type
    let types: Vec<&str> = response
        .transactions
        .iter()
        .map(|tx| tx.transaction_type.as_str())
        .collect();
    assert!(types.contains(&"Deposit"));
    assert!(types.contains(&"Withdrawal"));
    assert!(types.contains(&"TransferSent"));

    // Verify to_account received the transfer
    let to_result = client
        .get_transactions(&to_account.account_id, None, None)
        .await;
    assert_success(&to_result);
    let to_response = to_result.unwrap();
    assert_eq!(to_response.total, 1);
    assert_eq!(
        to_response.transactions[0].transaction_type,
        "TransferReceived"
    );
}

#[rstest]
#[tokio::test]
async fn get_transactions_pagination_default() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account
    let create_request = AccountFactory::default_jpy_account("Pagination Default User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Make deposits
    for _ in 0..5 {
        let deposit_request = TransactionFactory::deposit("100", "JPY");
        let deposit_result = client.deposit(&created.account_id, &deposit_request).await;
        assert_success(&deposit_result);
    }

    let result = client
        .get_transactions(&created.account_id, None, None)
        .await;

    assert_success(&result);
    let response = result.unwrap();
    assert_eq!(response.page, 1);
    assert_eq!(response.page_size, 20); // default
    assert_eq!(response.total, 5);
    assert_eq!(response.transactions.len(), 5);
}

#[rstest]
#[tokio::test]
async fn get_transactions_pagination_custom() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    // Create account
    let create_request = AccountFactory::default_jpy_account("Pagination Custom User");
    let create_result = client.create_account(&create_request).await;
    assert_success(&create_result);
    let created = create_result.unwrap();

    // Make 10 deposits
    for _ in 0..10 {
        let deposit_request = TransactionFactory::deposit("100", "JPY");
        let deposit_result = client.deposit(&created.account_id, &deposit_request).await;
        assert_success(&deposit_result);
    }

    // Get page 1 with page_size 3
    let result_page1 = client
        .get_transactions(&created.account_id, Some(1), Some(3))
        .await;
    assert_success(&result_page1);
    let page1 = result_page1.unwrap();
    assert_eq!(page1.page, 1);
    assert_eq!(page1.page_size, 3);
    assert_eq!(page1.total, 10);
    assert_eq!(page1.transactions.len(), 3);

    // Get page 2 with page_size 3
    let result_page2 = client
        .get_transactions(&created.account_id, Some(2), Some(3))
        .await;
    assert_success(&result_page2);
    let page2 = result_page2.unwrap();
    assert_eq!(page2.page, 2);
    assert_eq!(page2.transactions.len(), 3);

    // Get page 4 with page_size 3 (should have 1 transaction)
    let result_page4 = client
        .get_transactions(&created.account_id, Some(4), Some(3))
        .await;
    assert_success(&result_page4);
    let page4 = result_page4.unwrap();
    assert_eq!(page4.page, 4);
    assert_eq!(page4.transactions.len(), 1);

    // Get page 5 with page_size 3 (should be empty)
    let result_page5 = client
        .get_transactions(&created.account_id, Some(5), Some(3))
        .await;
    assert_success(&result_page5);
    let page5 = result_page5.unwrap();
    assert_eq!(page5.transactions.len(), 0);
}

#[rstest]
#[tokio::test]
async fn get_transactions_account_not_found() {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);
    let non_existent_id = non_existent_uuid();

    let result = client.get_transactions(&non_existent_id, None, None).await;

    assert_api_error(&result, "ACCOUNT_NOT_FOUND", StatusCode::NOT_FOUND);
}

#[rstest]
#[case("not-a-valid-uuid")]
#[case("12345")]
#[tokio::test]
async fn get_transactions_invalid_uuid(#[case] invalid_id: &str) {
    let client = BankApiClient::new(&DockerConfig::default().app_base_url);

    let result = client.get_transactions(invalid_id, None, None).await;

    assert_api_error(&result, "INVALID_ACCOUNT_ID", StatusCode::BAD_REQUEST);
}
