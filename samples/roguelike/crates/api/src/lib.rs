//! API layer for Dungeon of Pure Functions
//!
//! This crate provides HTTP endpoints using the Axum framework.
//! It transforms workflow results into HTTP responses and handles
//! all API-level concerns like DTOs and error responses.

pub mod dto;
pub mod errors;
pub mod handlers;
pub mod routes;
pub mod state;
