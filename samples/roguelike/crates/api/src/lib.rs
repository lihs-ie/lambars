//! API layer for Dungeon of Pure Functions
//!
//! This crate provides HTTP endpoints using the Axum framework.
//! It transforms workflow results into HTTP responses and handles
//! all API-level concerns like DTOs and error responses.
//!
//! # Modules
//!
//! - [`dto`]: Data Transfer Objects for request/response handling
//! - [`errors`]: API error types and HTTP response conversions
//! - [`handlers`]: HTTP request handlers
//! - [`middleware`]: HTTP middleware (request ID, response time)
//! - [`routes`]: Router configuration
//! - [`server`]: Server startup and configuration
//! - [`state`]: Application state management

pub mod dto;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;
pub mod state;
