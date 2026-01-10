//! Workflow layer for Dungeon of Pure Functions.
//!
//! This crate defines application use cases as functional workflows
//! using `AsyncIO`. It contains abstract port definitions (traits)
//! without concrete IO implementations.
//!
//! # Architecture
//!
//! The workflow layer sits between the domain layer and the infrastructure layer
//! in the onion architecture:
//!
//! ```text
//! +---------------------------------------------------+
//! |                   API Layer                       |
//! +---------------------------------------------------+
//! |               Infrastructure Layer                |
//! |  (Repository Impl, Cache Impl, External APIs)     |
//! +---------------------------------------------------+
//! |                Workflow Layer (this crate)        |
//! |  (Use Cases, Ports, Commands, Errors)             |
//! +---------------------------------------------------+
//! |                  Domain Layer                     |
//! |  (Entities, Value Objects, Domain Services)       |
//! +---------------------------------------------------+
//! ```
//!
//! # Modules
//!
//! - [`commands`]: Command types for workflow inputs
//! - [`errors`]: Error types for workflow operations
//! - [`ports`]: Abstract interfaces for infrastructure dependencies
//! - [`workflows`]: Use case implementations
//!
//! # Design Principles
//!
//! ## IO at the Edges
//!
//! All I/O operations (database, cache, external APIs) are represented as
//! `AsyncIO<T>` values. This defers side effects until the workflow's edge,
//! maintaining referential transparency within the pure workflow logic.
//!
//! ## Ports and Adapters
//!
//! External dependencies are abstracted as traits (ports). Concrete
//! implementations (adapters) are provided by the infrastructure layer.
//! This enables:
//! - Easy testing with mock implementations
//! - Swapping implementations without changing workflow logic
//! - Clear separation of concerns
//!
//! # Examples
//!
//! ```ignore
//! use roguelike_workflow::ports::{GameSessionRepository, RandomGenerator};
//! use roguelike_workflow::errors::WorkflowError;
//! use lambars::effect::AsyncIO;
//!
//! // Workflow function receives ports as dependencies
//! fn create_game_workflow<R, G>(
//!     repository: R,
//!     random: G,
//! ) -> impl Fn(CreateGameCommand) -> AsyncIO<Result<GameSession, WorkflowError>>
//! where
//!     R: GameSessionRepository,
//!     G: RandomGenerator,
//! {
//!     move |command| {
//!         // 1. [IO] Generate random seed
//!         // 2. [Pure] Create game session
//!         // 3. [IO] Save to repository
//!         // ...
//!     }
//! }
//! ```

pub mod commands;
pub mod errors;
pub mod ports;
pub mod workflows;

// Re-export common types for convenience
pub use errors::WorkflowError;
pub use ports::{
    EventStore, GameSessionRepository, RandomGenerator, SessionCache, SnapshotStore, WorkflowResult,
};

// Re-export workflow types
pub use workflows::game_session::{
    CreateGameCommand, CreateSnapshotCommand, EndGameCommand, ResumeGameCommand, create_game,
    create_snapshot, end_game, reconstruct_from_events, resume_game,
};

// Re-export player workflow types
pub use workflows::player::{
    AttackEnemyCommand, EquipItemCommand, MovePlayerCommand, PickUpItemCommand, TakeDamageCommand,
    UseItemCommand, attack_enemy, equip_item, move_player, pick_up_item, take_damage, use_item,
};
