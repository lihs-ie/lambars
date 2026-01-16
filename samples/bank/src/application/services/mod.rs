//! Application services for the bank sample.
//!
//! This module contains reusable services that support the application layer.
//!
//! # Available Services
//!
//! - [`event_replay`] - Stack-safe event replay using Trampoline

pub mod event_replay;

pub use event_replay::*;
