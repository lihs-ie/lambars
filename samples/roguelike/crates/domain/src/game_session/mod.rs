//! Game session domain module.
//!
//! This module provides the core types for managing game sessions in the
//! roguelike game. A game session represents a single playthrough from
//! start to finish.
//!
//! # Overview
//!
//! The game session domain includes:
//!
//! - **Identifiers**: Unique identifiers for game sessions
//! - **Status**: Game state tracking (in progress, victory, defeat, paused)
//! - **Events**: Domain events for game lifecycle
//! - **Errors**: Error types for game session operations
//!
//! # Examples
//!
//! ```
//! use roguelike_domain::game_session::{
//!     GameIdentifier, GameStatus, GameOutcome,
//!     GameStarted, GameEnded, RandomSeed,
//! };
//!
//! // Create a new game session identifier
//! let identifier = GameIdentifier::new();
//!
//! // Track game status
//! let status = GameStatus::InProgress;
//! assert!(status.is_active());
//! assert!(!status.is_terminal());
//!
//! // Create domain events
//! let started = GameStarted::new(identifier, RandomSeed::new(12345));
//! let ended = GameEnded::new(GameOutcome::Victory);
//! ```
//!
//! # Types
//!
//! - [`GameIdentifier`]: Unique identifier for game sessions (UUID newtype)
//! - [`GameStatus`] / [`GameOutcome`]: Game state and outcome enums
//! - [`GameStarted`], [`GameEnded`], [`TurnStarted`], [`TurnEnded`]: Domain events
//! - [`GameSessionError`]: Error types for game session operations
//! - [`RandomSeed`]: Random seed for game reproducibility

mod errors;
mod events;
mod identifier;
mod status;

// Re-export identifier types
pub use identifier::GameIdentifier;

// Re-export status types
pub use status::{GameOutcome, GameStatus};

// Re-export error types
pub use errors::GameSessionError;

// Re-export event types
pub use events::{GameEnded, GameSessionEvent, GameStarted, RandomSeed, TurnEnded, TurnStarted};
