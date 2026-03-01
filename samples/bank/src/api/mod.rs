//! API layer for the Bank sample application.
//!
//! This module provides HTTP endpoints using Axum 0.8 for the bank application.
//! It follows functional programming principles:
//!
//! - **Pure Functions**: DTO transformations are pure functions
//! - **Bifunctor**: Error transformations use bifunctor-like patterns
//! - **Iso**: Bidirectional DTO conversions where applicable
//!
//! # Architecture
//!
//! ```text
//! HTTP Request
//!     │
//!     ▼
//! ┌───────────────┐
//! │   Handlers    │ ── Extract request, validate, call workflow
//! └───────────────┘
//!     │
//!     ▼
//! ┌───────────────┐
//! │  Transformers │ ── DTO ↔ Domain conversion (pure functions)
//! └───────────────┘
//!     │
//!     ▼
//! ┌───────────────┐
//! │   Workflows   │ ── Business logic (Application layer)
//! └───────────────┘
//!     │
//!     ▼
//! HTTP Response
//! ```
//!
//! # Modules
//!
//! - [`dto`]: Data Transfer Objects for requests and responses
//! - [`handlers`]: Axum handlers for HTTP endpoints
//! - [`middleware`]: Error handling and other middleware
//! - [`routes`]: Route configuration

pub mod dto;
pub mod handlers;
pub mod middleware;
pub mod routes;

pub use routes::create_router;
