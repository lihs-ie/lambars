//! Domain layer for Dungeon of Pure Functions
//!
//! This crate contains all domain entities, value objects, aggregates,
//! domain events, and domain services. All logic is implemented as pure
//! functions without side effects.

pub mod aggregates;
pub mod entities;
pub mod errors;
pub mod events;
pub mod services;
pub mod values;
