//! API error handling and response conversion.
//!
//! This module provides error types and conversions for the API layer:
//!
//! - [`ApiError`]: The main error type for API handlers
//! - [`conversion`]: Conversions from domain/workflow errors

pub mod api_error;
pub mod conversion;

pub use api_error::ApiError;
