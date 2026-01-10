//! Player workflow module.
//!
//! This module provides workflow implementations for managing player actions
//! using functional programming patterns with `AsyncIO`.
//!
//! # Overview
//!
//! The player workflow handles:
//! - Moving the player in the dungeon
//! - Attacking enemies
//! - Using items from inventory
//! - Picking up items from the floor
//! - Equipping items to slots
//! - Taking damage from enemies or traps
//!
//! # Architecture
//!
//! All workflows follow the "IO at the Edges" pattern:
//! - Pure domain logic is isolated in dedicated functions
//! - IO operations (repository, cache, event store) are deferred via `AsyncIO`
//! - Dependencies are injected via higher-order functions
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::workflows::player::{
//!     move_player, MovePlayerCommand,
//! };
//! use roguelike_domain::common::Direction;
//!
//! // Create the workflow function with dependencies
//! let workflow = move_player(&cache, &event_store);
//!
//! // Execute the workflow
//! let command = MovePlayerCommand::new(game_identifier, Direction::Up);
//! let result = workflow(command).run_async().await;
//! ```

mod attack_enemy;
mod commands;
mod equip_item;
mod move_player;
mod pick_up_item;
mod take_damage;
mod use_item;

// Re-export command types
pub use commands::{
    AttackEnemyCommand, EquipItemCommand, MovePlayerCommand, PickUpItemCommand, TakeDamageCommand,
    UseItemCommand,
};

// Re-export workflow functions
pub use attack_enemy::attack_enemy;
pub use equip_item::equip_item;
pub use move_player::move_player;
pub use pick_up_item::pick_up_item;
pub use take_damage::take_damage;
pub use use_item::use_item;
