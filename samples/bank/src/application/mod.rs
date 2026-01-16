//! Application layer for the bank sample.
//!
//! This module contains the application logic, including:
//!
//! - **Queries**: Read-side operations for retrieving data
//! - **Validation**: Input validation using Applicative patterns
//! - **Workflows**: Pure functions for business operations
//! - **Services**: Reusable application services
//!
//! # Design Principles
//!
//! All code in this layer follows functional programming principles:
//!
//! - **Pure Functions**: All workflow functions are pure (no side effects)
//! - **Immutability**: Data flows through transformations without mutation
//! - **Either for Errors**: Errors are represented using `Either<DomainError, T>`
//! - **Composition**: Workflows are composed from smaller pure functions

pub mod queries;
pub mod services;
pub mod validation;
pub mod workflows;

pub use queries::*;
pub use services::*;
pub use validation::*;
pub use workflows::*;
