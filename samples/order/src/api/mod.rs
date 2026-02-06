//! API module
//!
//! Defines functions and types that serve as HTTP API entry points.
//!
//! # Module Structure
//!
//! - [`types`] - HTTP request/response types
//! - [`dependencies`] - Dummy dependency functions
//! - [`place_order_api`] - `PlaceOrder` API endpoint
//! - [`axum_handler`] - Handler for the axum framework
//!
//! # Design Principles
//!
//! - All API functions return `IO<HttpResponse>`
//! - Dependency functions are injected as arguments (for testability)
//! - DTO-to-domain type conversions are pure functions
//! - The axum handler calls `run_unsafe()` on IO as the "edge of the world"

pub mod axum_handler;
pub mod dependencies;
pub mod place_order_api;
pub mod types;

// Re-exports
pub use dependencies::{
    calculate_shipping_cost, check_address_exists, check_product_exists,
    create_acknowledgment_letter, get_pricing_function, send_acknowledgment,
};
pub use place_order_api::place_order_api;
pub use types::{HttpRequest, HttpResponse};
