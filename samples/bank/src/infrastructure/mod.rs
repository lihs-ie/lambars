//! Infrastructure layer for the bank application.
//!
//! This module contains all infrastructure concerns including:
//!
//! - **Configuration**: Application settings loaded from environment variables
//! - **Event Store**: PostgreSQL-based event sourcing storage
//! - **Read Model**: Redis-based caching for query side
//! - **Messaging**: SQS message handling for event publication
//! - **Dependencies**: Dependency injection container
//!
//! # Design Principles
//!
//! The infrastructure layer follows these principles:
//!
//! - **Trait-based abstraction**: All external dependencies are abstracted behind traits
//!   for testability and flexibility
//! - **Pure functions for transformations**: Data transformations (e.g., event to message)
//!   are implemented as pure functions
//! - **`AsyncIO` for side effects**: All I/O operations return `AsyncIO` to defer execution
//! - **Immutable data structures**: Uses `PersistentList`, `PersistentHashMap` where appropriate
//!
//! # Module Organization
//!
//! - `config` - Application configuration with lazy loading
//! - `event_store` - Event sourcing storage abstraction
//! - `read_model` - Read model cache abstraction
//! - `messaging` - Message queue integration
//! - `dependencies` - Dependency injection container

mod config;
mod dependencies;
mod event_store;
mod messaging;
mod read_model;

pub use config::{AppConfig, ConfigError};
pub use dependencies::AppDependencies;
pub use event_store::{EventStore, EventStoreError, PostgresEventStore};
pub use messaging::{EventMessage, event_to_message, events_to_messages};
pub use read_model::{CachedBalance, ReadModelCache, ReadModelError};
