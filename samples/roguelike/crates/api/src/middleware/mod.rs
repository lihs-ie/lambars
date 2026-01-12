//! API middleware components.
//!
//! This module provides middleware for the API layer:
//!
//! - [`request_id`]: Request ID extraction and injection
//! - [`response_time`]: Response time logging

pub mod request_id;
pub mod response_time;

pub use request_id::{RequestId, RequestIdLayer};
pub use response_time::ResponseTimeLayer;
