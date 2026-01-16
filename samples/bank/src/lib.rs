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
//! - Smart constructors for validation

pub mod domain;
