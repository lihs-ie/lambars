//! Infrastructure module for external services.
//!
//! This module contains database repositories, cache implementations,
//! and other infrastructure concerns.

pub mod in_memory;
pub mod postgres;
pub mod redis;
pub mod repository;

pub use in_memory::{InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository};
pub use postgres::{PostgresEventStore, PostgresProjectRepository, PostgresTaskRepository};
pub use redis::{RedisProjectRepository, RedisTaskRepository};
pub use repository::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};
