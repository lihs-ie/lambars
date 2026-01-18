//! Infrastructure module for external services.
//!
//! This module contains database repositories, cache implementations,
//! and other infrastructure concerns.

pub mod in_memory;
pub mod repository;

pub use in_memory::{InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository};
pub use repository::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};
