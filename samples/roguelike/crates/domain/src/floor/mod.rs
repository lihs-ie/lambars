//! Floor domain module for dungeon structures.
//!
//! This module provides types for representing and manipulating dungeon floors:
//!
//! - **Aggregates**: `Floor` as the aggregate root for a dungeon level
//! - **Identifiers**: `FloorIdentifier` for unique floor identification
//! - **Tiles**: `Tile`, `TileKind`, `TrapType` for floor tiles
//! - **Rooms**: `Room` for rectangular rooms with validation
//! - **Corridors**: `Corridor` for passages between rooms
//! - **Errors**: `FloorError` for floor-related errors
//! - **Events**: Domain events for floor operations
//!
//! All types follow functional programming principles:
//! - Immutability: All operations return new values
//! - Validation: Constructors return Result for constrained types
//! - Type safety: Newtype pattern for identifiers

mod aggregate;
mod corridor;
mod errors;
mod events;
mod identifier;
mod room;
mod tile;

// Re-export aggregate types
pub use aggregate::Floor;

// Re-export identifier types
pub use identifier::FloorIdentifier;

// Re-export tile types
pub use tile::{Tile, TileKind, TrapType};

// Re-export room types
pub use room::Room;

// Re-export corridor types
pub use corridor::Corridor;

// Re-export error types
pub use errors::FloorError;

// Re-export event types
pub use events::{FloorEntered, TileExplored, TrapTriggered};
