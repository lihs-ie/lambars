//! Data Transfer Objects for the API layer.
//!
//! This module defines DTOs for API requests and responses, as well as
//! transformation functions between DTOs and domain types.
//!
//! # Design Principles
//!
//! - **Separation**: DTOs are separate from domain types
//! - **Validation**: Input validation happens during transformation
//! - **Pure Functions**: All transformations are pure functions
//! - **Bifunctor Pattern**: Error transformations use `map_left`/`map_right`

pub mod requests;
pub mod responses;
pub mod transformers;

pub use requests::{
    DepositRequest, MoneyDto, OpenAccountRequest, TransferRequest, WithdrawRequest,
};
pub use responses::{
    AccountResponse, BalanceResponse, MoneyResponseDto, TransactionHistoryResponse,
    TransactionRecordDto, TransactionResponse,
};
pub use transformers::{
    account_to_response, dto_to_money, event_to_transaction_response, money_to_dto,
};
