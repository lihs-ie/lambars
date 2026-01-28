//! Application services for the bank sample.
//!
//! This module contains reusable services that support the application layer.
//!
//! # Available Services
//!
//! - [`event_replay`] - Stack-safe event replay using Trampoline
//! - [`idempotency`] - Idempotency checking for transaction operations

pub mod event_replay;
pub mod idempotency;

pub use event_replay::*;
pub use idempotency::*;
