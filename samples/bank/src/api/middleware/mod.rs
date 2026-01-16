//! Middleware components for the API layer.
//!
//! This module provides cross-cutting concerns for the API:
//!
//! - Error handling and transformation
//! - Request/response logging (future)
//! - Authentication (future)

pub mod error_handler;

pub use error_handler::{ApiError, domain_error_to_api_error};
