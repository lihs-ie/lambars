//! Workflow layer for Dungeon of Pure Functions
//!
//! This crate defines application use cases as functional workflows
//! using AsyncIO. It contains abstract port definitions (traits)
//! without concrete IO implementations.

pub mod commands;
pub mod errors;
pub mod ports;
pub mod workflows;
