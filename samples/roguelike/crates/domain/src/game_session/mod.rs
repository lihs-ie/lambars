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
//! - **Aggregate**: GameSession as the aggregate root for a game playthrough
//! - **Identifiers**: Unique identifiers for game sessions
//! - **Status**: Game state tracking (in progress, victory, defeat, paused)
//! - **Events**: Domain events for game lifecycle
//! - **Errors**: Error types for game session operations
//!
//! # Examples
//!
//! ```
//! use roguelike_domain::common::{
//!     Attack, BaseStats, CombatStats, Defense, FloorLevel, Health,
//!     Mana, Position, Speed, Stat,
//! };
//! use roguelike_domain::player::{Player, PlayerIdentifier, PlayerName};
//! use roguelike_domain::floor::{Floor, FloorIdentifier};
//! use roguelike_domain::game_session::{
//!     GameSession, GameIdentifier, GameStatus, GameOutcome,
//!     GameStarted, GameEnded, RandomSeed,
//! };
//!
//! // Create a player
//! let player = Player::new(
//!     PlayerIdentifier::new(),
//!     PlayerName::new("Hero").unwrap(),
//!     Position::new(5, 5),
//!     CombatStats::new(
//!         Health::new(100).unwrap(),
//!         Health::new(100).unwrap(),
//!         Mana::new(50).unwrap(),
//!         Mana::new(50).unwrap(),
//!         Attack::new(20),
//!         Defense::new(15),
//!         Speed::new(10),
//!     ).unwrap(),
//!     BaseStats::new(
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!         Stat::new(10).unwrap(),
//!     ),
//! );
//!
//! // Create a floor
//! let floor = Floor::new(
//!     FloorIdentifier::new(1),
//!     FloorLevel::new(1).unwrap(),
//!     80,
//!     40,
//! );
//!
//! // Create a game session
//! let session = GameSession::new(
//!     GameIdentifier::new(),
//!     player,
//!     floor,
//!     RandomSeed::new(12345),
//! );
//!
//! assert!(session.is_active());
//! assert!(!session.is_terminal());
//!
//! // Create domain events
//! let started = GameStarted::new(*session.identifier(), RandomSeed::new(12345));
//! let ended = GameEnded::new(GameOutcome::Victory);
//! ```
//!
//! # Types
//!
//! - [`GameSession`]: The aggregate root for a game session
//! - [`GameIdentifier`]: Unique identifier for game sessions (UUID newtype)
//! - [`GameStatus`] / [`GameOutcome`]: Game state and outcome enums
//! - [`GameStarted`], [`GameEnded`], [`TurnStarted`], [`TurnEnded`]: Domain events
//! - [`GameSessionError`]: Error types for game session operations
//! - [`RandomSeed`]: Random seed for game reproducibility

mod aggregate;
mod errors;
mod events;
mod identifier;
mod status;

// Re-export aggregate
pub use aggregate::GameSession;

// Re-export identifier types
pub use identifier::GameIdentifier;

// Re-export status types
pub use status::{GameOutcome, GameStatus};

// Re-export error types
pub use errors::GameSessionError;

// Re-export event types
pub use events::{GameEnded, GameSessionEvent, GameStarted, RandomSeed, TurnEnded, TurnStarted};
