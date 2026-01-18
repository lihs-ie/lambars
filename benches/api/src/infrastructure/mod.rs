//! Infrastructure module for external services.
//!
//! This module contains database repositories, cache implementations,
//! and other infrastructure concerns.

pub mod repository;

pub use repository::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};
