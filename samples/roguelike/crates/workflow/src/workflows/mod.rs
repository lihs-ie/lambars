//! Workflow implementations.
//!
//! This module provides the application use case implementations
//! for the roguelike game. Each workflow follows the "IO at the Edges"
//! pattern, separating pure domain logic from IO operations.
//!
//! # Modules
//!
//! - [`game_session`]: Game session lifecycle workflows
//! - [`player`]: Player action workflows
//! - [`enemy`]: Enemy action workflows
//! - [`floor`]: Floor management workflows
//! - [`turn`]: Turn processing workflows

pub mod game_session;

// Pure helper functions in player module are prepared for future GameSession integration
#[allow(dead_code)]
pub mod player;

// Pure helper functions in enemy module are prepared for future GameSession integration
#[allow(dead_code)]
pub mod enemy;

// Pure helper functions in floor module are prepared for future GameSession integration
#[allow(dead_code)]
pub mod floor;

// Pure helper functions in turn module are prepared for future GameSession integration
#[allow(dead_code)]
pub mod turn;
