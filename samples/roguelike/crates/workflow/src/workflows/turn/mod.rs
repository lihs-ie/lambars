//! Turn workflow module.
//!
//! This module provides workflow implementations for managing game turns
//! using functional programming patterns with `AsyncIO`.
//!
//! # Overview
//!
//! The turn workflow handles:
//! - Processing player turns (movement, attack, item usage)
//! - Processing all enemy turns in speed order
//! - Applying status effects at turn end
//! - Checking game over conditions (victory/defeat)
//! - Wait/rest turns for player recovery
//!
//! # Architecture
//!
//! All workflows follow the "IO at the Edges" pattern:
//! - Pure domain logic is isolated in dedicated functions
//! - IO operations (cache, event store) are deferred via `AsyncIO`
//! - Dependencies are injected via higher-order functions
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::turn::{
//!     process_turn, ProcessTurnCommand, PlayerCommand,
//! };
//! use roguelike_domain::common::Direction;
//!
//! // Create the workflow function with dependencies
//! let workflow = process_turn(&cache, &event_store, &snapshot_store, cache_ttl);
//!
//! // Execute the workflow with a player move command
//! let command = ProcessTurnCommand::new(
//!     game_identifier,
//!     PlayerCommand::Move(Direction::Up),
//! );
//! let result = workflow(command).run_async().await;
//! ```

mod commands;
mod process_turn;
mod wait_turn;

// Re-export command types
pub use commands::{PlayerCommand, ProcessTurnCommand, WaitTurnCommand};

// Re-export result types
pub use process_turn::TurnResult;

// Re-export workflow functions
pub use process_turn::process_turn;
pub use wait_turn::wait_turn;

// Re-export pure functions for testing and composition
pub use process_turn::{
    EntityTurnOrder, apply_status_effect_tick, check_game_over, end_turn, execute_player_command,
    process_all_enemy_turns, process_status_effects, resolve_turn_order, start_turn,
    validate_player_command,
};
pub use wait_turn::{
    WaitBonus, apply_wait_bonus, calculate_hp_regeneration, can_benefit_from_wait,
};
