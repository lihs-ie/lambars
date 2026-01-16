//! HTTP handlers for the Bank API.
//!
//! This module provides Axum handlers that:
//!
//! - Extract and validate request data
//! - Transform DTOs to domain types
//! - Execute workflows
//! - Transform results back to DTOs
//!
//! # Functional Approach
//!
//! Handlers follow a pipeline pattern:
//!
//! ```text
//! Request → Extract → Validate → Transform → Execute → Transform → Response
//! ```
//!
//! Each step is a pure function except for the Execute step which
//! runs the `AsyncIO` computation.

pub mod account;
pub mod transaction;

pub use account::{create_account, get_account, get_balance};
pub use transaction::{deposit_handler, get_transactions, transfer_handler, withdraw_handler};
