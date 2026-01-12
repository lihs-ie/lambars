//! MySQL adapters.
//!
//! This module provides MySQL connection pool management for the infrastructure layer.
//!
//! # Components
//!
//! - [`MySqlPoolConfig`]: Configuration for MySQL connection pool settings
//! - [`MySqlPool`]: A wrapper around `sqlx::MySqlPool` with Arc-based sharing
//! - [`MySqlPoolFactory`]: Factory for creating MySQL connection pools
//!
//! # Examples
//!
//! ```rust,ignore
//! use roguelike_infrastructure::adapters::mysql::{MySqlPoolConfig, MySqlPoolFactory};
//!
//! // Create a pool configuration
//! let config = MySqlPoolConfig::with_url("mysql://user:password@localhost:3306/database");
//!
//! // Create a pool synchronously
//! let pool = MySqlPoolFactory::create_pool(&config)?;
//!
//! // Or create a pool asynchronously using AsyncIO
//! let async_pool = MySqlPoolFactory::create_pool_async(&config);
//! let pool = async_pool.run_async().await?;
//! ```

mod config;
mod factory;
mod pool;
mod repository;

pub use config::MySqlPoolConfig;
pub use factory::MySqlPoolFactory;
pub use pool::MySqlPool;
pub use repository::{GameSessionRecord, MySqlGameSessionRepository};
