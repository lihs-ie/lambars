//! Enemy workflow module.
//!
//! This module provides workflow implementations for managing enemy actions
//! using functional programming patterns with `AsyncIO`.
//!
//! # Overview
//!
//! The enemy workflow handles:
//! - Processing enemy turns (AI decision making and action execution)
//! - Spawning enemies on floors
//! - Processing enemy deaths (loot drops, removal from session)
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
//! use roguelike_workflow::workflows::enemy::{
//!     process_enemy_turn, ProcessEnemyTurnCommand,
//! };
//!
//! // Create the workflow function with dependencies
//! let workflow = process_enemy_turn(&cache, &event_store, cache_ttl);
//!
//! // Execute the workflow
//! let command = ProcessEnemyTurnCommand::new(game_identifier, entity_identifier);
//! let result = workflow(command).run_async().await;
//! ```

mod commands;
mod process_enemy_death;
mod process_enemy_turn;
mod spawn_enemies;

// Re-export command types
pub use commands::{ProcessEnemyDeathCommand, ProcessEnemyTurnCommand, SpawnEnemiesCommand};

// Re-export EnemyAction type
pub use process_enemy_turn::EnemyAction;

// Re-export workflow functions
pub use process_enemy_death::process_enemy_death;
pub use process_enemy_turn::process_enemy_turn;
pub use spawn_enemies::spawn_enemies;

// Re-export pure functions for testing and composition
pub use process_enemy_death::{calculate_loot, drop_items_at_position, remove_enemy_from_session};
pub use process_enemy_turn::{
    decide_enemy_action, execute_enemy_action, find_enemy_by_id, validate_enemy_active,
};
pub use spawn_enemies::{
    add_enemies_to_session, find_valid_spawn_points, generate_enemies, get_spawn_configuration,
};
