//! Floor workflow module.
//!
//! This module provides workflow implementations for managing dungeon floors
//! using functional programming patterns with `AsyncIO`.
//!
//! # Overview
//!
//! The floor workflow handles:
//! - Generating new floors (rooms, corridors, stairs, items, traps)
//! - Descending to the next floor
//! - Updating tile visibility based on field of view
//! - Triggering and processing trap effects
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
//! use roguelike_workflow::workflows::floor::{
//!     generate_floor, GenerateFloorCommand,
//! };
//!
//! // Create the workflow function with dependencies
//! let workflow = generate_floor(&cache, &event_store, cache_ttl);
//!
//! // Execute the workflow
//! let command = GenerateFloorCommand::new(game_identifier, 1);
//! let result = workflow(command).run_async().await;
//! ```

mod commands;
mod descend_floor;
mod generate_floor;
mod trigger_trap;
mod update_visibility;

// Re-export command types
pub use commands::{
    DescendFloorCommand, GenerateFloorCommand, TriggerTrapCommand, UpdateVisibilityCommand,
};

// Re-export workflow functions
pub use descend_floor::descend_floor;
pub use generate_floor::generate_floor;
pub use trigger_trap::trigger_trap;
pub use update_visibility::update_visibility;

// Re-export configuration types
pub use generate_floor::FloorGenerationConfiguration;
pub use trigger_trap::TrapEffect;

// Re-export pure functions for testing and composition
pub use descend_floor::{
    calculate_next_floor_level, set_player_at_spawn_point, spawn_floor_enemies,
    update_session_for_floor_change, validate_at_down_stairs,
};
pub use generate_floor::{
    get_floor_configuration, place_items, place_stairs, place_traps, update_session_floor,
};
pub use trigger_trap::{
    TrapInfo, apply_trap_effect, calculate_trap_effect, disarm_trap, find_trap_at_position,
};
pub use update_visibility::{
    calculate_field_of_view, get_player_position, update_explored_tiles, update_session_visibility,
};
