//! Bank Sample Application
//!
//! Event Sourcing / CQRS sample using lambars library.
//!
//! This application demonstrates the use of the lambars library for functional
//! programming in Rust, combined with Event Sourcing and CQRS patterns.
//!
//! # Architecture
//!
//! The application follows the Onion Architecture:
//!
//! - **Domain Layer**: Pure business logic, value objects, aggregates, events
//! - **Application Layer**: Commands, queries, workflows, validation
//! - **Infrastructure Layer**: Database, cache, messaging
//! - **API Layer**: HTTP handlers, DTOs, middleware
//!
//! # lambars Features Used
//!
//! - `Either` for error handling
//! - `Semigroup` and `Monoid` for composing values (Money)
//! - `Trampoline` for stack-safe event replay
//! - `PersistentList` for immutable event sequences
//! - Smart constructors for validation
//!
//! # Workflow Pattern
//!
//! Workflows are pure functions that compose validation, business logic,
//! and event generation:
//!
//! ```text
//! Command → Validate → Transform → Event
//! ```
//!
//! Each step is a pure function:
//! - Errors propagate using `Either<DomainError, T>`
//! - No side effects in core logic
//! - I/O is isolated at the boundaries

pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;
