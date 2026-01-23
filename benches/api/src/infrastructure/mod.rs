//! Infrastructure module for external services.
//!
//! This module contains database repositories, cache implementations,
//! benchmark scenario configuration, and other infrastructure concerns.

pub mod factory;
pub mod in_memory;
pub mod postgres;
pub mod redis;
pub mod repository;
pub mod scenario;

pub use factory::{
    CacheMode, ConfigurationError, FactoryError, Repositories, RepositoryConfig,
    RepositoryConfigBuilder, RepositoryFactory, StorageMode,
};
pub use in_memory::{InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository};
pub use postgres::{PostgresEventStore, PostgresProjectRepository, PostgresTaskRepository};
pub use redis::{RedisProjectRepository, RedisTaskRepository};
pub use repository::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, TaskRepository,
};
pub use scenario::{
    BenchmarkScenario, BenchmarkScenarioBuilder, CacheMetricsConfig, CacheState, ConcurrencyConfig,
    ContentionLevel, DataScale, DataScaleConfig, ExtendableScenario, LoadPattern, PartialScenario,
    PayloadVariant, PoolConfig, ProfilingConfig, RpsProfile, ScenarioError, ScenarioMatrix,
    ScenarioRegistry, ScenarioTemplate, ScenarioValidation, Thresholds, WorkerConfig,
};
