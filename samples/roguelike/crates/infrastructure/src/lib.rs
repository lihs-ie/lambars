//! Infrastructure layer for Dungeon of Pure Functions
//!
//! This crate provides concrete implementations of the ports
//! defined in the workflow layer. It handles all external I/O
//! including MySQL, Redis, and file operations.

pub mod adapters;
pub mod config;
pub mod errors;
