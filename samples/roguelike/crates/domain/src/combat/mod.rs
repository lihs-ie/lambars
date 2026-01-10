//! Combat domain module for Dungeon of Pure Functions.
//!
//! This module contains the combat system, including damage calculation,
//! turn resolution, and combat-related error types.
//!
//! # Overview
//!
//! The combat system is designed as a set of pure functions that handle:
//!
//! - **Damage Calculation**: Using Semigroup for modifier composition
//!   and pipe! for calculation pipelines
//! - **Turn Resolution**: Using PersistentTreeMap for speed-based ordering
//! - **Combat Errors**: Comprehensive error types for combat failures
//!
//! # Example
//!
//! ```
//! use roguelike_domain::combat::CombatError;
//!
//! // Check if a target is in range before attacking
//! fn validate_attack_range(
//!     attacker_position: (i32, i32),
//!     target_position: (i32, i32),
//!     attack_range: u32,
//! ) -> Result<(), CombatError> {
//!     let (ax, ay) = attacker_position;
//!     let (tx, ty) = target_position;
//!     let distance = ((tx - ax).abs() + (ty - ay).abs()) as u32;
//!
//!     if distance <= attack_range {
//!         Ok(())
//!     } else {
//!         Err(CombatError::target_not_in_range(
//!             attacker_position,
//!             target_position,
//!             attack_range,
//!         ))
//!     }
//! }
//! ```

pub mod errors;

pub use errors::CombatError;
