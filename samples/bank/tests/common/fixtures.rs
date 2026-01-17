//! Test data factories for integration tests.

use super::client::{
    DepositRequest, MoneyDto, OpenAccountRequest, TransferRequest, WithdrawRequest,
};
use uuid::Uuid;

pub struct AccountFactory;

impl AccountFactory {
    pub fn create_request(
        owner_name: &str,
        initial_balance: &str,
        currency: &str,
    ) -> OpenAccountRequest {
        OpenAccountRequest {
            owner_name: owner_name.to_string(),
            initial_balance: MoneyDto {
                amount: initial_balance.to_string(),
                currency: currency.to_string(),
            },
        }
    }

    pub fn default_jpy_account(owner_name: &str) -> OpenAccountRequest {
        Self::create_request(owner_name, "10000", "JPY")
    }

    pub fn zero_balance_account(owner_name: &str) -> OpenAccountRequest {
        Self::create_request(owner_name, "0", "JPY")
    }
}

pub struct TransactionFactory;

impl TransactionFactory {
    pub fn deposit(amount: &str, currency: &str) -> DepositRequest {
        DepositRequest {
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: generate_idempotency_key(),
        }
    }

    pub fn deposit_with_key(amount: &str, currency: &str, idempotency_key: &str) -> DepositRequest {
        DepositRequest {
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: idempotency_key.to_string(),
        }
    }

    pub fn withdraw(amount: &str, currency: &str) -> WithdrawRequest {
        WithdrawRequest {
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: generate_idempotency_key(),
        }
    }

    pub fn withdraw_with_key(
        amount: &str,
        currency: &str,
        idempotency_key: &str,
    ) -> WithdrawRequest {
        WithdrawRequest {
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: idempotency_key.to_string(),
        }
    }

    pub fn transfer(to_account_id: &str, amount: &str, currency: &str) -> TransferRequest {
        TransferRequest {
            to_account_id: to_account_id.to_string(),
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: generate_idempotency_key(),
        }
    }

    pub fn transfer_with_key(
        to_account_id: &str,
        amount: &str,
        currency: &str,
        idempotency_key: &str,
    ) -> TransferRequest {
        TransferRequest {
            to_account_id: to_account_id.to_string(),
            amount: MoneyDto {
                amount: amount.to_string(),
                currency: currency.to_string(),
            },
            idempotency_key: idempotency_key.to_string(),
        }
    }
}

pub fn generate_idempotency_key() -> String {
    format!("test-{}", Uuid::now_v7())
}

pub fn non_existent_uuid() -> String {
    Uuid::now_v7().to_string()
}
