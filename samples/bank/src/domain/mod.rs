//! Domain layer for the bank application.
//!
//! The domain layer contains the core business logic and is completely
//! independent of infrastructure concerns. It follows Domain-Driven Design
//! principles and functional programming patterns.
//!
//! # Structure
//!
//! - [`value_objects`] - Immutable values that describe domain concepts
//! - [`account`] - Account aggregate and related types
//! - `dsl` - Domain-specific language for banking operations (to be implemented)
//!
//! # Design Principles
//!
//! All code in this layer adheres to:
//!
//! - **Referential transparency**: Functions always return the same output for the same input
//! - **Pure functions**: No side effects (I/O, state mutation)
//! - **Immutability**: Data structures are never modified in place

pub mod account;
pub mod value_objects;

pub use account::*;
pub use value_objects::*;
