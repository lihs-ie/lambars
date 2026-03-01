//! Query module for read-side operations.
//!
//! This module contains pure query functions that transform domain data
//! into read-optimized response types. All functions are pure and have no side effects.
//!
//! # Available Queries
//!
//! - [`get_balance()`] - Get the current balance of an account
//! - [`build_transaction_history()`] - Get transaction history for an account
//!
//! # Design Principles
//!
//! - **Pure Functions**: All query functions are pure (no side effects)
//! - **Immutability**: Data flows through transformations without mutation
//! - **Type Safety**: Strong typing for all query inputs and outputs
//! - **Serde Support**: All response types implement Serialize/Deserialize

mod get_balance;
mod get_history;

pub use get_balance::{BalanceResponse, GetBalanceQuery, get_balance};
pub use get_history::{
    GetHistoryQuery, TransactionHistory, TransactionRecord, TransactionType,
    build_transaction_history, event_to_transaction_record,
};
