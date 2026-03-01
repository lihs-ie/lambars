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
//! Handlers follow a pipeline pattern using lambars pipe!/compose! macros:
//!
//! ```text
//! Request → Extract → Validate → Transform → Execute → Transform → Response
//! ```
//!
//! Each step is a pure function except for the Execute step which
//! runs the `AsyncIO` computation.
//!
//! # Pipeline Utilities
//!
//! The [`pipeline`] module provides utilities for composing handler operations
//! using functional patterns. See [`pipeline`] for details.
//!
//! # Workflow Composition with eff_async!
//!
//! The [`workflow_eff`] module provides utilities for composing async workflows
//! using the `ExceptT` monad transformer with `eff_async!` macro for do-notation
//! style error handling.

pub mod account;
pub mod pipeline;
pub mod transaction;
pub mod workflow_eff;

pub use account::{create_account, get_account, get_balance};
pub use transaction::{
    deposit_handler, deposit_handler_eff, get_transactions, transfer_handler, withdraw_handler,
    withdraw_handler_eff,
};
