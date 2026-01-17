//! Integration tests for the Bank API.
//!
//! These tests require the Docker environment to be running:
//!
//! ```bash
//! cd samples/bank/docker && docker compose up -d
//! ```
//!
//! Run tests with:
//!
//! ```bash
//! cargo test --test integration_tests
//! ```

mod api;
mod common;
