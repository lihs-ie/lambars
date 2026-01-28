//! Infrastructure module for external services.
//!
//! This module contains database repositories, cache implementations,
//! benchmark scenario configuration, and other infrastructure concerns.

pub mod cache;
pub mod external;
pub mod factory;
pub mod fail_injection;
pub mod in_memory;
pub mod postgres;
pub mod redis;
pub mod repository;
pub mod scenario;

pub use cache::{
    CacheConfig, CacheResult, CacheStatus, CacheStrategy, CachedProjectRepository,
    CachedTaskRepository, project_data_key, project_latest_key, task_data_key, task_latest_key,
};
pub use external::{
    ExternalDataSource, ExternalError, ExternalSources, ExternalTaskData, HttpExternalDataSource,
    RedisExternalDataSource, StubExternalDataSource,
};
pub use factory::{
    CacheMode, ConfigurationError, FactoryError, Repositories, RepositoryConfig,
    RepositoryConfigBuilder, RepositoryFactory, StorageMode,
};
pub use fail_injection::{
    ConfigError, EnvParseError, FailInjectionConfig, RngProvider, ScopedRng, apply_post_injection,
};
pub use in_memory::{InMemoryEventStore, InMemoryProjectRepository, InMemoryTaskRepository};
pub use postgres::{PostgresEventStore, PostgresProjectRepository, PostgresTaskRepository};
pub use redis::{RedisProjectRepository, RedisTaskRepository};
pub use repository::{
    EventStore, PaginatedResult, Pagination, ProjectRepository, RepositoryError, SearchScope,
    TaskRepository,
};
pub use scenario::{
    BenchmarkScenario, BenchmarkScenarioBuilder, CacheMetricsConfig, CacheState, ConcurrencyConfig,
    ContentionLevel, DataScale, DataScaleConfig, ExtendableScenario, LoadPattern, PartialScenario,
    PayloadVariant, PoolConfig, ProfilingConfig, RpsProfile, ScenarioError, ScenarioMatrix,
    ScenarioRegistry, ScenarioTemplate, ScenarioValidation, Thresholds, WorkerConfig,
};
