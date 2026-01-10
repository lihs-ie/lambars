//! Game session workflow module.
//!
//! This module provides workflow implementations for managing game session
//! lifecycle using functional programming patterns with `AsyncIO`.
//!
//! # Overview
//!
//! The game session workflow handles:
//! - Creating new game sessions
//! - Resuming existing sessions (with Event Sourcing)
//! - Ending sessions
//! - Creating periodic snapshots
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
//! use roguelike_workflow::workflows::game_session::{
//!     create_game, CreateGameCommand,
//! };
//!
//! // Create the workflow function with dependencies
//! let workflow = create_game(&repository, &event_store, &cache, &random);
//!
//! // Execute the workflow
//! let command = CreateGameCommand::new("Player Name".to_string(), None);
//! let result = workflow(command).run_async().await;
//! ```

mod commands;
mod create_game;
mod end_game;
mod resume_game;
mod snapshot;

// Re-export command types
pub use commands::{CreateGameCommand, CreateSnapshotCommand, EndGameCommand, ResumeGameCommand};

// Re-export workflow functions
pub use create_game::create_game;
pub use end_game::end_game;
pub use resume_game::{reconstruct_from_events, resume_game};
pub use snapshot::{
    DEFAULT_SNAPSHOT_INTERVAL, create_snapshot, create_snapshot_with_default_interval,
};
