//! HTTP client wrapper for integration tests.

use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::Duration;

#[derive(Clone)]
pub struct BankApiClient {
    client: Client,
    base_url: String,
}

impl BankApiClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    // Health check
    pub async fn health(&self) -> ApiResult<HealthResponse> {
        self.get("/health").await
    }

    // Account operations
    pub async fn create_account(&self, request: &OpenAccountRequest) -> ApiResult<AccountResponse> {
        self.post("/accounts", request).await
    }

    pub async fn get_account(&self, account_id: &str) -> ApiResult<AccountResponse> {
        self.get(&format!("/accounts/{account_id}")).await
    }

    pub async fn get_balance(&self, account_id: &str) -> ApiResult<BalanceResponse> {
        self.get(&format!("/accounts/{account_id}/balance")).await
    }

    // Transaction operations
    pub async fn deposit(
        &self,
        account_id: &str,
        request: &DepositRequest,
    ) -> ApiResult<TransactionResponse> {
        self.post(&format!("/accounts/{account_id}/deposit"), request)
            .await
    }

    pub async fn withdraw(
        &self,
        account_id: &str,
        request: &WithdrawRequest,
    ) -> ApiResult<TransactionResponse> {
        self.post(&format!("/accounts/{account_id}/withdraw"), request)
            .await
    }

    pub async fn transfer(
        &self,
        account_id: &str,
        request: &TransferRequest,
    ) -> ApiResult<TransferResponse> {
        self.post(&format!("/accounts/{account_id}/transfer"), request)
            .await
    }

    pub async fn get_transactions(
        &self,
        account_id: &str,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> ApiResult<TransactionHistoryResponse> {
        let mut url = format!("/accounts/{account_id}/transactions");
        let mut params = vec![];
        if let Some(p) = page {
            params.push(format!("page={p}"));
        }
        if let Some(ps) = page_size {
            params.push(format!("page_size={ps}"));
        }
        if !params.is_empty() {
            url = format!("{url}?{}", params.join("&"));
        }
        self.get(&url).await
    }

    // Internal helpers
    async fn get<T: DeserializeOwned>(&self, path: &str) -> ApiResult<T> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await?;
        parse_response(response).await
    }

    #[allow(clippy::future_not_send)]
    async fn post<T: DeserializeOwned, R: Serialize>(&self, path: &str, body: &R) -> ApiResult<T> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?;
        parse_response(response).await
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Api { status: StatusCode, code: String },
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err)
    }
}

async fn parse_response<T: DeserializeOwned>(response: Response) -> ApiResult<T> {
    let status = response.status();

    if status.is_success() {
        response.json().await.map_err(ApiError::Http)
    } else {
        let error_body: ApiErrorBody = response.json().await.map_err(ApiError::Http)?;
        Err(ApiError::Api {
            status,
            code: error_body.code,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    code: String,
}

// DTO types for tests

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyDto {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAccountRequest {
    pub owner_name: String,
    pub initial_balance: MoneyDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct DepositRequest {
    pub amount: MoneyDto,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WithdrawRequest {
    pub amount: MoneyDto,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferRequest {
    pub to_account_id: String,
    pub amount: MoneyDto,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AccountResponse {
    pub account_id: String,
    pub owner_name: String,
    pub balance: MoneyDto,
    pub status: String,
}

impl PartialEq for MoneyDto {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount && self.currency == other.currency
    }
}

impl Eq for MoneyDto {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BalanceResponse {
    pub account_id: String,
    pub balance: MoneyDto,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TransactionResponse {
    pub transaction_id: String,
    pub amount: MoneyDto,
    pub balance_after: MoneyDto,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TransferResponse {
    pub transfer_id: String,
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount: MoneyDto,
    pub from_balance_after: MoneyDto,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TransactionRecordDto {
    pub transaction_id: String,
    pub transaction_type: String,
    pub amount: MoneyDto,
    pub balance_after: MoneyDto,
    pub counterparty: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TransactionHistoryResponse {
    pub account_id: String,
    pub transactions: Vec<TransactionRecordDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
