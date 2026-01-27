//! Benchmark scenario configuration.
//!
//! This module provides data structures for defining benchmark scenarios
//! with configurable storage modes, cache modes, load patterns, and data scales.
//!
//! # Example
//!
//! ```yaml
//! name: "read_heavy_warm_postgres"
//! description: "Read-heavy workload with warm cache on PostgreSQL"
//! storage_mode: postgres
//! cache_mode: redis
//! load_pattern: read_heavy
//! cache_state: warm
//! data_scale: medium
//! duration_seconds: 60
//! connections: 10
//! threads: 2
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{CacheMode, StorageMode};

// =============================================================================
// Load Pattern
// =============================================================================

/// Load pattern for benchmark scenarios.
///
/// Defines the read/write ratio of the workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadPattern {
    /// Primarily read operations (90% read, 10% write).
    #[default]
    ReadHeavy,
    /// Primarily write operations (10% read, 90% write).
    WriteHeavy,
    /// Balanced read and write operations (50% read, 50% write).
    Mixed,
}

impl LoadPattern {
    /// Returns the read ratio as a percentage (0-100).
    #[must_use]
    pub const fn read_ratio(&self) -> u8 {
        match self {
            Self::ReadHeavy => 90,
            Self::WriteHeavy => 10,
            Self::Mixed => 50,
        }
    }

    /// Returns the write ratio as a percentage (0-100).
    #[must_use]
    pub const fn write_ratio(&self) -> u8 {
        match self {
            Self::ReadHeavy => 10,
            Self::WriteHeavy => 90,
            Self::Mixed => 50,
        }
    }
}

impl FromStr for LoadPattern {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "read_heavy" | "readheavy" | "read-heavy" => Ok(Self::ReadHeavy),
            "write_heavy" | "writeheavy" | "write-heavy" => Ok(Self::WriteHeavy),
            "mixed" | "balanced" => Ok(Self::Mixed),
            _ => Err(ScenarioError::InvalidLoadPattern(value.to_string())),
        }
    }
}

impl std::fmt::Display for LoadPattern {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadHeavy => write!(formatter, "read_heavy"),
            Self::WriteHeavy => write!(formatter, "write_heavy"),
            Self::Mixed => write!(formatter, "mixed"),
        }
    }
}

// =============================================================================
// Cache State
// =============================================================================

/// Cache state for benchmark scenarios.
///
/// Defines whether the cache is pre-populated (warm) or empty (cold).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheState {
    /// Cache is empty (all requests are cache misses).
    Cold,
    /// Cache is pre-populated with data (requests hit the cache).
    #[default]
    Warm,
}

impl FromStr for CacheState {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "cold" | "miss" | "empty" => Ok(Self::Cold),
            "warm" | "hit" | "populated" => Ok(Self::Warm),
            _ => Err(ScenarioError::InvalidCacheState(value.to_string())),
        }
    }
}

impl std::fmt::Display for CacheState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cold => write!(formatter, "cold"),
            Self::Warm => write!(formatter, "warm"),
        }
    }
}

// =============================================================================
// Data Scale
// =============================================================================

/// Data scale for benchmark scenarios.
///
/// Defines the size of the dataset used in benchmarks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataScale {
    /// Small dataset (~100 records).
    Small,
    /// Medium dataset (~10,000 records).
    #[default]
    Medium,
    /// Large dataset (~1,000,000 records).
    Large,
}

impl DataScale {
    /// Returns the approximate number of records for this scale.
    #[must_use]
    pub const fn record_count(&self) -> usize {
        match self {
            Self::Small => 100,
            Self::Medium => 10_000,
            Self::Large => 1_000_000,
        }
    }

    /// Returns the default record count for seeding data at this scale.
    ///
    /// This is the standard record count used when seeding benchmark data:
    /// - Small: 1,000 records
    /// - Medium: 10,000 records
    /// - Large: 1,000,000 records
    #[must_use]
    pub const fn default_record_count(&self) -> u64 {
        match self {
            Self::Small => 1_000,
            Self::Medium => 10_000,
            Self::Large => 1_000_000,
        }
    }
}

impl FromStr for DataScale {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "small" | "s" | "1e2" => Ok(Self::Small),
            "medium" | "m" | "1e4" => Ok(Self::Medium),
            "large" | "l" | "1e6" => Ok(Self::Large),
            _ => Err(ScenarioError::InvalidDataScale(value.to_string())),
        }
    }
}

impl std::fmt::Display for DataScale {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Small => write!(formatter, "small"),
            Self::Medium => write!(formatter, "medium"),
            Self::Large => write!(formatter, "large"),
        }
    }
}

// =============================================================================
// Data Scale Configuration
// =============================================================================

/// Extended data scale configuration for benchmark scenarios.
///
/// Provides fine-grained control over data seeding with support for:
/// - Custom record counts
/// - Reproducible data generation via random seeds
/// - Incremental seeding
///
/// # Example
///
/// ```yaml
/// data_scale_config:
///   scale: large
///   record_count: 500000  # Override default 1M
///   seed: 42              # Reproducible data
///   incremental: false
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataScaleConfig {
    /// Data scale level.
    pub scale: DataScale,

    /// Number of records to seed (overrides scale default if specified).
    #[serde(default)]
    pub record_count: Option<u64>,

    /// Random seed for reproducible data generation.
    #[serde(default)]
    pub seed: Option<u64>,

    /// Enable incremental seeding (add to existing data).
    #[serde(default)]
    pub incremental: bool,
}

impl DataScaleConfig {
    /// Creates a new `DataScaleConfig` with the given scale.
    #[must_use]
    pub const fn new(scale: DataScale) -> Self {
        Self {
            scale,
            record_count: None,
            seed: None,
            incremental: false,
        }
    }

    /// Creates a new `DataScaleConfig` with a specific record count.
    #[must_use]
    pub const fn with_record_count(scale: DataScale, record_count: u64) -> Self {
        Self {
            scale,
            record_count: Some(record_count),
            seed: None,
            incremental: false,
        }
    }

    /// Gets the effective record count.
    ///
    /// Returns the specified `record_count` if set, otherwise returns
    /// the default record count for the configured scale.
    #[must_use]
    pub fn effective_record_count(&self) -> u64 {
        self.record_count
            .unwrap_or_else(|| self.scale.default_record_count())
    }

    /// Sets the random seed for reproducible data generation.
    #[must_use]
    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Enables incremental seeding.
    #[must_use]
    pub const fn with_incremental(mut self, incremental: bool) -> Self {
        self.incremental = incremental;
        self
    }
}

impl Default for DataScaleConfig {
    fn default() -> Self {
        Self::new(DataScale::default())
    }
}

impl From<DataScale> for DataScaleConfig {
    fn from(scale: DataScale) -> Self {
        Self::new(scale)
    }
}

// =============================================================================
// Payload Variant
// =============================================================================

/// Payload variant for benchmark scenarios.
///
/// Defines the complexity of request/response payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadVariant {
    /// Minimal payload (0 tags, 0 subtasks).
    Minimal,
    /// Standard payload (10 tags, 10 subtasks).
    #[default]
    Standard,
    /// Complex payload (100 tags, 50 subtasks).
    Complex,
    /// Heavy payload (100 tags, 200 subtasks).
    Heavy,
}

impl PayloadVariant {
    /// Returns the number of tags for this variant.
    #[must_use]
    pub const fn tag_count(&self) -> usize {
        match self {
            Self::Minimal => 0,
            Self::Standard => 10,
            Self::Complex | Self::Heavy => 100,
        }
    }

    /// Returns the number of subtasks for this variant.
    #[must_use]
    pub const fn subtask_count(&self) -> usize {
        match self {
            Self::Minimal => 0,
            Self::Standard => 10,
            Self::Complex => 50,
            Self::Heavy => 200,
        }
    }
}

impl FromStr for PayloadVariant {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "minimal" | "min" | "tiny" => Ok(Self::Minimal),
            "standard" | "std" | "normal" => Ok(Self::Standard),
            "complex" => Ok(Self::Complex),
            "heavy" | "max" | "large" => Ok(Self::Heavy),
            _ => Err(ScenarioError::InvalidPayloadVariant(value.to_string())),
        }
    }
}

impl std::fmt::Display for PayloadVariant {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minimal => write!(formatter, "minimal"),
            Self::Standard => write!(formatter, "standard"),
            Self::Complex => write!(formatter, "complex"),
            Self::Heavy => write!(formatter, "heavy"),
        }
    }
}

// =============================================================================
// RPS Profile
// =============================================================================

/// RPS (Requests Per Second) profile for load generation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpsProfile {
    /// Constant RPS throughout the test.
    #[default]
    Constant,
    /// Ramp up from 0 to target RPS, then ramp down.
    RampUpDown,
    /// Sudden spike in traffic.
    Burst,
    /// Gradually increasing RPS (stress test).
    StepUp,
}

impl FromStr for RpsProfile {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "constant" | "steady" | "flat" => Ok(Self::Constant),
            "ramp_up_down" | "rampupdown" | "ramp" => Ok(Self::RampUpDown),
            "burst" | "spike" => Ok(Self::Burst),
            "step_up" | "stepup" | "stress" => Ok(Self::StepUp),
            _ => Err(ScenarioError::InvalidRpsProfile(value.to_string())),
        }
    }
}

impl std::fmt::Display for RpsProfile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant => write!(formatter, "constant"),
            Self::RampUpDown => write!(formatter, "ramp_up_down"),
            Self::Burst => write!(formatter, "burst"),
            Self::StepUp => write!(formatter, "step_up"),
        }
    }
}

// =============================================================================
// Benchmark Scenario
// =============================================================================

/// A complete benchmark scenario configuration.
///
/// This struct represents a YAML scenario file and contains all parameters
/// needed to configure and run a benchmark.
///
/// **Important**: All core configuration fields are required to prevent silent
/// fallback to defaults due to typos. Use `deny_unknown_fields` to catch
/// unrecognized field names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkScenario {
    /// Unique name for this scenario (required).
    pub name: String,
    /// Human-readable description (required).
    pub description: String,

    // Backend configuration (required)
    /// Storage mode (`in_memory`, `postgres`).
    pub storage_mode: StorageMode,
    /// Cache mode (`in_memory`, `redis`).
    pub cache_mode: CacheMode,

    // Workload configuration (required)
    /// Load pattern (`read_heavy`, `write_heavy`, `mixed`).
    pub load_pattern: LoadPattern,
    /// Cache state (`cold`, `warm`).
    pub cache_state: CacheState,
    /// Data scale (`small`, `medium`, `large`).
    pub data_scale: DataScale,
    /// Payload variant (`minimal`, `standard`, `complex`, `heavy`).
    pub payload_variant: PayloadVariant,
    /// RPS profile (`constant`, `ramp_up_down`, `burst`, `step_up`).
    pub rps_profile: RpsProfile,

    // Load generation parameters (optional with defaults)
    /// Test duration in seconds.
    #[serde(default = "default_duration_seconds")]
    pub duration_seconds: u64,
    /// Number of concurrent connections.
    #[serde(default = "default_connections")]
    pub connections: u32,
    /// Number of threads.
    #[serde(default = "default_threads")]
    pub threads: u32,
    /// Target RPS (0 = unlimited).
    #[serde(default)]
    pub target_rps: u32,
    /// Warmup duration in seconds.
    #[serde(default = "default_warmup_seconds")]
    pub warmup_seconds: u64,

    // Target endpoints (empty = all endpoints)
    #[serde(default)]
    pub endpoints: Vec<String>,

    /// HTTP methods for the benchmark (e.g., `POST`, `GET`).
    /// Used when endpoints need specific HTTP methods.
    #[serde(default)]
    pub http_methods: Vec<String>,

    // Pool configuration overrides
    #[serde(default)]
    pub pool_sizes: Option<PoolConfig>,

    // Worker configuration overrides
    #[serde(default)]
    pub worker_config: Option<WorkerConfig>,

    /// Performance thresholds for validation.
    #[serde(default)]
    pub thresholds: Option<Thresholds>,

    /// Additional metadata for result analysis.
    #[serde(default)]
    pub metadata: Option<ScenarioMetadata>,

    /// Concurrency configuration for stress testing.
    #[serde(default)]
    pub concurrency: Option<ConcurrencyConfig>,

    /// Contention level for resource conflict scenarios.
    ///
    /// Defaults to `ContentionLevel::Low` if not specified.
    #[serde(default)]
    pub contention_level: ContentionLevel,

    /// Profiling configuration for perf and flamegraph integration.
    #[serde(default)]
    pub profiling: ProfilingConfig,

    /// Extended data scale configuration (optional).
    ///
    /// When specified, provides fine-grained control over data seeding
    /// with custom record counts, random seeds, and incremental seeding.
    /// If not specified, `effective_data_scale_config()` will generate
    /// a default configuration based on `data_scale`.
    #[serde(default)]
    pub data_scale_config: Option<DataScaleConfig>,

    /// Cache metrics configuration for tracking cache hit/miss behavior.
    #[serde(default)]
    pub cache_metrics: CacheMetricsConfig,

    /// Error handling and timeout configuration.
    #[serde(default)]
    pub error_config: ErrorConfig,

    /// Custom environment variables passed to the API server.
    ///
    /// These variables are exported before starting the API server,
    /// allowing scenario-specific configuration like cache settings.
    ///
    /// # Example
    ///
    /// ```yaml
    /// environment:
    ///   CACHE_ENABLED: "true"
    ///   CACHE_STRATEGY: "read-through"
    ///   CACHE_TTL_SECS: "60"
    /// ```
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

const fn default_duration_seconds() -> u64 {
    60
}

const fn default_connections() -> u32 {
    10
}

const fn default_threads() -> u32 {
    2
}

const fn default_warmup_seconds() -> u64 {
    5
}

impl Default for BenchmarkScenario {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            storage_mode: StorageMode::default(),
            cache_mode: CacheMode::default(),
            load_pattern: LoadPattern::default(),
            cache_state: CacheState::default(),
            data_scale: DataScale::default(),
            payload_variant: PayloadVariant::default(),
            rps_profile: RpsProfile::default(),
            duration_seconds: default_duration_seconds(),
            connections: default_connections(),
            threads: default_threads(),
            target_rps: 0,
            warmup_seconds: default_warmup_seconds(),
            endpoints: Vec::new(),
            http_methods: Vec::new(),
            pool_sizes: None,
            worker_config: None,
            thresholds: None,
            metadata: None,
            concurrency: None,
            contention_level: ContentionLevel::default(),
            profiling: ProfilingConfig::default(),
            data_scale_config: None,
            cache_metrics: CacheMetricsConfig::default(),
            error_config: ErrorConfig::default(),
            environment: HashMap::new(),
        }
    }
}

impl BenchmarkScenario {
    /// Creates a new scenario builder.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> BenchmarkScenarioBuilder {
        BenchmarkScenarioBuilder::new(name)
    }

    /// Loads a scenario from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ScenarioError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| ScenarioError::FileRead(error.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parses a scenario from YAML content.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the YAML is invalid.
    pub fn from_yaml(content: &str) -> Result<Self, ScenarioError> {
        serde_yaml::from_str(content).map_err(|error| ScenarioError::YamlParse(error.to_string()))
    }

    /// Serializes the scenario to YAML.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if serialization fails.
    pub fn to_yaml(&self) -> Result<String, ScenarioError> {
        serde_yaml::to_string(self).map_err(|error| ScenarioError::YamlSerialize(error.to_string()))
    }

    /// Gets the effective data scale configuration.
    ///
    /// Returns the `data_scale_config` if specified, otherwise generates
    /// a default configuration based on `data_scale`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, DataScale, DataScaleConfig,
    /// };
    ///
    /// // Without data_scale_config, uses data_scale to generate default
    /// let scenario = BenchmarkScenario::builder("test")
    ///     .description("Test scenario")
    ///     .data_scale(DataScale::Large)
    ///     .build();
    ///
    /// let config = scenario.effective_data_scale_config();
    /// assert_eq!(config.scale, DataScale::Large);
    /// assert_eq!(config.effective_record_count(), 1_000_000);
    /// ```
    #[must_use]
    pub fn effective_data_scale_config(&self) -> DataScaleConfig {
        self.data_scale_config
            .clone()
            .unwrap_or_else(|| DataScaleConfig::new(self.data_scale))
    }

    /// Generates a canonical scenario name from all configuration parameters.
    ///
    /// The name includes all distinguishing parameters to prevent collision
    /// when storing results from different configurations, including contention
    /// level and concurrency configuration (if present).
    #[must_use]
    pub fn canonical_name(&self) -> String {
        let base = format!(
            "{}_{}_{}_{}_{}_{}_{}",
            self.storage_mode,
            self.cache_mode,
            self.load_pattern,
            self.cache_state,
            self.data_scale,
            self.payload_variant,
            self.rps_profile
        );

        // Add contention level
        let contention_suffix = format!("_{}", self.contention_level);

        // Add concurrency configuration if present
        // For preset configurations (small, medium, large), use the preset name
        // For custom configurations, include all values to prevent collision
        let concurrency_suffix = self
            .concurrency
            .as_ref()
            .map(|config| {
                let preset = config.preset_name();
                if preset == "custom" {
                    // Include all configuration values to prevent collision between different custom configs
                    format!(
                        "_w{}d{}r{}m{}",
                        config.worker_threads,
                        config.database_pool_size,
                        config.redis_pool_size,
                        config.max_connections
                    )
                } else {
                    format!("_{preset}")
                }
            })
            .unwrap_or_default();

        format!("{base}{contention_suffix}{concurrency_suffix}")
    }

    /// Generates a short canonical name (storage/cache/load/state only).
    ///
    /// Use this when a shorter identifier is needed and other parameters
    /// are consistent across comparisons.
    #[must_use]
    pub fn short_canonical_name(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.storage_mode, self.cache_mode, self.load_pattern, self.cache_state
        )
    }

    /// Generates environment variables for wrk/benchmark execution.
    ///
    /// These environment variables are consumed by Lua scripts (e.g., `contention.lua`)
    /// and shell scripts (e.g., `profile.sh`) to configure benchmark behavior based on
    /// scenario settings.
    ///
    /// # Environment Variables
    ///
    /// ## Contention Settings
    /// - `CONTENTION_LEVEL`: "low", "medium", or "high"
    /// - `WRITE_RATIO`: Write operation ratio (0-100)
    /// - `TARGET_RESOURCES`: Number of resources to target based on contention level
    ///
    /// ## Concurrency Settings (if concurrency config present)
    /// - `WORKER_THREADS`: Number of Axum worker threads
    /// - `DATABASE_POOL_SIZE`: Database connection pool size
    /// - `REDIS_POOL_SIZE`: Redis connection pool size
    /// - `MAX_CONNECTIONS`: Maximum simultaneous connections
    ///
    /// ## Load Generation Parameters
    /// - `CONNECTIONS`: Number of concurrent connections
    /// - `THREADS`: Number of threads
    /// - `DURATION_SECONDS`: Test duration
    /// - `TARGET_RPS`: Target requests per second (if > 0)
    ///
    /// ## Profiling Settings
    /// - `ENABLE_PERF`: "1" if perf recording is enabled
    /// - `ENABLE_FLAMEGRAPH`: "1" if flamegraph generation is enabled
    /// - `PERF_FREQUENCY`: Sampling frequency in Hz
    /// - `PROFILING_OUTPUT_DIR`: Output directory for profiling results
    ///
    /// ## Data Scale Settings
    /// - `DATA_SCALE`: Data scale level (small, medium, large)
    /// - `RECORD_COUNT`: Effective record count for seeding
    /// - `RANDOM_SEED`: Random seed for reproducible data (if specified)
    /// - `INCREMENTAL`: "1" if incremental seeding is enabled
    ///
    /// ## Error Configuration
    /// - `REQUEST_TIMEOUT_MS`: Request timeout in milliseconds
    /// - `CONNECT_TIMEOUT_MS`: Connection timeout in milliseconds
    /// - `MAX_RETRIES`: Maximum retry attempts
    /// - `RETRY_DELAY_MS`: Delay between retries in milliseconds
    /// - `EXPECTED_ERROR_RATE`: Expected error rate threshold (if specified)
    /// - `FAIL_ON_ERROR_THRESHOLD`: "1" if test should fail when error rate exceeds threshold
    /// - `INJECT_ERROR_RATE`: Error injection rate for chaos testing (if specified)
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, ConcurrencyConfig, ContentionLevel, StorageMode, CacheMode,
    ///     LoadPattern, CacheState, DataScale, PayloadVariant, RpsProfile,
    /// };
    ///
    /// let scenario = BenchmarkScenario::builder("high_contention_test")
    ///     .description("High contention scenario")
    ///     .storage_mode(StorageMode::Postgres)
    ///     .cache_mode(CacheMode::Redis)
    ///     .load_pattern(LoadPattern::WriteHeavy)
    ///     .cache_state(CacheState::Warm)
    ///     .data_scale(DataScale::Medium)
    ///     .payload_variant(PayloadVariant::Standard)
    ///     .rps_profile(RpsProfile::Constant)
    ///     .concurrency(ConcurrencyConfig::small_pool())
    ///     .contention_level(ContentionLevel::High)
    ///     .build();
    ///
    /// let env_vars = scenario.to_env_vars();
    ///
    /// // Use with std::process::Command:
    /// // let mut cmd = Command::new("wrk");
    /// // for (key, value) in &env_vars {
    /// //     cmd.env(key, value);
    /// // }
    /// ```
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        let mut env_vars = Vec::new();

        // Contention level settings
        env_vars.push((
            "CONTENTION_LEVEL".to_string(),
            self.contention_level.to_string(),
        ));
        env_vars.push((
            "WRITE_RATIO".to_string(),
            self.contention_level.write_ratio().to_string(),
        ));
        env_vars.push((
            "TARGET_RESOURCES".to_string(),
            self.contention_level.target_resource_count().to_string(),
        ));

        // Concurrency configuration (if present)
        if let Some(ref concurrency) = self.concurrency {
            env_vars.push((
                "WORKER_THREADS".to_string(),
                concurrency.worker_threads.to_string(),
            ));
            env_vars.push((
                "DATABASE_POOL_SIZE".to_string(),
                concurrency.database_pool_size.to_string(),
            ));
            env_vars.push((
                "REDIS_POOL_SIZE".to_string(),
                concurrency.redis_pool_size.to_string(),
            ));
            env_vars.push((
                "MAX_CONNECTIONS".to_string(),
                concurrency.max_connections.to_string(),
            ));
        }

        // Load generation parameters
        env_vars.push(("CONNECTIONS".to_string(), self.connections.to_string()));
        env_vars.push(("THREADS".to_string(), self.threads.to_string()));
        env_vars.push((
            "DURATION_SECONDS".to_string(),
            self.duration_seconds.to_string(),
        ));

        if self.target_rps > 0 {
            env_vars.push(("TARGET_RPS".to_string(), self.target_rps.to_string()));
        }

        // Profiling configuration
        if self.profiling.enable_perf {
            env_vars.push(("ENABLE_PERF".to_string(), "1".to_string()));
        }
        if self.profiling.enable_flamegraph {
            env_vars.push(("ENABLE_FLAMEGRAPH".to_string(), "1".to_string()));
        }
        env_vars.push((
            "PERF_FREQUENCY".to_string(),
            self.profiling.frequency.to_string(),
        ));
        env_vars.push((
            "PROFILING_OUTPUT_DIR".to_string(),
            self.profiling.output_dir.clone(),
        ));

        // Data scale configuration
        let data_scale_config = self.effective_data_scale_config();
        env_vars.push((
            "DATA_SCALE".to_string(),
            data_scale_config.scale.to_string(),
        ));
        env_vars.push((
            "RECORD_COUNT".to_string(),
            data_scale_config.effective_record_count().to_string(),
        ));
        if let Some(seed) = data_scale_config.seed {
            env_vars.push(("RANDOM_SEED".to_string(), seed.to_string()));
        }
        if data_scale_config.incremental {
            env_vars.push(("INCREMENTAL".to_string(), "1".to_string()));
        }

        // Cache metrics configuration (only output if enabled)
        if self.cache_metrics.enabled {
            env_vars.push(("CACHE_METRICS_ENABLED".to_string(), "1".to_string()));
            if self.cache_metrics.per_endpoint {
                env_vars.push(("CACHE_METRICS_PER_ENDPOINT".to_string(), "1".to_string()));
            }
            if self.cache_metrics.track_latency {
                env_vars.push(("CACHE_METRICS_TRACK_LATENCY".to_string(), "1".to_string()));
            }
            env_vars.push((
                "CACHE_WARMUP_REQUESTS".to_string(),
                self.cache_metrics.warmup_requests.to_string(),
            ));
            if let Some(rate) = self.cache_metrics.expected_hit_rate {
                env_vars.push(("EXPECTED_CACHE_HIT_RATE".to_string(), rate.to_string()));
            }
        }

        // Error configuration
        env_vars.push((
            "REQUEST_TIMEOUT_MS".to_string(),
            self.error_config.timeout_ms.to_string(),
        ));
        env_vars.push((
            "CONNECT_TIMEOUT_MS".to_string(),
            self.error_config.connect_timeout_ms.to_string(),
        ));
        env_vars.push((
            "MAX_RETRIES".to_string(),
            self.error_config.max_retries.to_string(),
        ));
        env_vars.push((
            "RETRY_DELAY_MS".to_string(),
            self.error_config.retry_delay_ms.to_string(),
        ));
        if let Some(rate) = self.error_config.expected_error_rate {
            env_vars.push(("EXPECTED_ERROR_RATE".to_string(), rate.to_string()));
        }
        if self.error_config.fail_on_error_threshold {
            env_vars.push(("FAIL_ON_ERROR_THRESHOLD".to_string(), "1".to_string()));
        }
        if let Some(rate) = self.error_config.inject_error_rate {
            env_vars.push(("INJECT_ERROR_RATE".to_string(), rate.to_string()));
        }

        env_vars
    }

    /// Generates cache-related environment variables from the scenario.
    ///
    /// This method consolidates environment variables from multiple sources:
    /// 1. The `environment` section (direct key-value pairs)
    /// 2. The `cache_metrics` section (converted to environment variables)
    ///
    /// # Returns
    ///
    /// A `HashMap` containing all cache-related environment variables.
    /// Keys from the `environment` section take precedence over generated values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, StorageMode, CacheMode, LoadPattern, CacheState,
    ///     DataScale, PayloadVariant, RpsProfile, CacheMetricsConfig,
    /// };
    /// use std::collections::HashMap;
    ///
    /// let mut env = HashMap::new();
    /// env.insert("CACHE_ENABLED".to_string(), "true".to_string());
    /// env.insert("CACHE_STRATEGY".to_string(), "read-through".to_string());
    /// env.insert("CACHE_TTL_SECS".to_string(), "60".to_string());
    ///
    /// let scenario = BenchmarkScenario::builder("cache_test")
    ///     .description("Cache test scenario")
    ///     .storage_mode(StorageMode::Postgres)
    ///     .cache_mode(CacheMode::Redis)
    ///     .load_pattern(LoadPattern::ReadHeavy)
    ///     .cache_state(CacheState::Warm)
    ///     .data_scale(DataScale::Medium)
    ///     .payload_variant(PayloadVariant::Standard)
    ///     .rps_profile(RpsProfile::Constant)
    ///     .cache_metrics(CacheMetricsConfig::warm_cache(1000))
    ///     .environment(env)
    ///     .build();
    ///
    /// let cache_env = scenario.get_cache_environment();
    ///
    /// assert_eq!(cache_env.get("CACHE_ENABLED"), Some(&"true".to_string()));
    /// assert_eq!(cache_env.get("CACHE_STRATEGY"), Some(&"read-through".to_string()));
    /// assert_eq!(cache_env.get("CACHE_WARMUP_REQUESTS"), Some(&"1000".to_string()));
    /// ```
    #[must_use]
    pub fn get_cache_environment(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Generate variables from cache_metrics configuration
        if self.cache_metrics.enabled {
            env.insert("CACHE_METRICS_ENABLED".to_string(), "1".to_string());
            env.insert(
                "CACHE_WARMUP_REQUESTS".to_string(),
                self.cache_metrics.warmup_requests.to_string(),
            );
            if let Some(rate) = self.cache_metrics.expected_hit_rate {
                env.insert("EXPECTED_CACHE_HIT_RATE".to_string(), rate.to_string());
            }
            // per_endpoint and track_latency flags
            env.insert(
                "CACHE_METRICS_PER_ENDPOINT".to_string(),
                if self.cache_metrics.per_endpoint {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
            env.insert(
                "CACHE_METRICS_TRACK_LATENCY".to_string(),
                if self.cache_metrics.track_latency {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            );
        }

        // Apply environment section (takes precedence)
        env.extend(self.environment.clone());

        env
    }
}

/// Pool configuration for database and cache connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Database connection pool size.
    #[serde(default = "default_db_pool_size")]
    pub database_pool_size: u32,
    /// Redis connection pool size.
    #[serde(default = "default_redis_pool_size")]
    pub redis_pool_size: u32,
}

const fn default_db_pool_size() -> u32 {
    10
}

const fn default_redis_pool_size() -> u32 {
    10
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            database_pool_size: default_db_pool_size(),
            redis_pool_size: default_redis_pool_size(),
        }
    }
}

// =============================================================================
// Profiling Configuration
// =============================================================================

/// Profiling configuration for benchmark scenarios.
///
/// Enables integration with perf and flamegraph for detailed performance analysis.
///
/// # Example
///
/// ```yaml
/// profiling:
///   enable_perf: true
///   enable_flamegraph: true
///   frequency: 99
///   output_dir: "profiling-results"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfilingConfig {
    /// Enable perf recording during benchmark execution.
    pub enable_perf: bool,
    /// Enable flamegraph generation from perf data.
    pub enable_flamegraph: bool,
    /// Sampling frequency in Hz for perf recording.
    ///
    /// Default is 99 Hz to avoid lockstep with timer interrupts.
    pub frequency: u32,
    /// Output directory for profiling results.
    pub output_dir: String,
}

const fn default_profiling_frequency() -> u32 {
    99
}

fn default_profiling_output_dir() -> String {
    "profiling-results".to_string()
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enable_perf: false,
            enable_flamegraph: false,
            frequency: default_profiling_frequency(),
            output_dir: default_profiling_output_dir(),
        }
    }
}

impl ProfilingConfig {
    /// Creates a profiling configuration with perf enabled.
    #[must_use]
    pub fn with_perf() -> Self {
        Self {
            enable_perf: true,
            ..Default::default()
        }
    }

    /// Creates a profiling configuration with both perf and flamegraph enabled.
    #[must_use]
    pub fn with_flamegraph() -> Self {
        Self {
            enable_perf: true,
            enable_flamegraph: true,
            ..Default::default()
        }
    }

    /// Returns true if any profiling is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enable_perf || self.enable_flamegraph
    }
}

// =============================================================================
// Cache Metrics Configuration
// =============================================================================

/// Cache behavior metrics configuration.
///
/// Enables tracking of cache hit/miss rates, latency distribution,
/// and per-endpoint cache metrics for benchmark scenarios.
///
/// # Example
///
/// ```yaml
/// cache_metrics:
///   enabled: true
///   per_endpoint: true
///   track_latency: true
///   warmup_requests: 1000
///   expected_hit_rate: 0.8
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheMetricsConfig {
    /// Enable cache hit/miss tracking.
    #[serde(default = "CacheMetricsConfig::default_enabled")]
    pub enabled: bool,

    /// Track cache hit rate per endpoint.
    #[serde(default)]
    pub per_endpoint: bool,

    /// Track cache latency distribution.
    #[serde(default)]
    pub track_latency: bool,

    /// Cache warmup requests before measurement.
    #[serde(default)]
    pub warmup_requests: u32,

    /// Expected cache hit rate threshold (for alerting).
    #[serde(default)]
    pub expected_hit_rate: Option<f64>,
}

// Manual Default implementation is intentional to keep consistency with
// CacheMetricsConfig::default_enabled() used for serde deserialization.
#[allow(clippy::derivable_impls)]
impl Default for CacheMetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default to avoid overhead in existing scenarios
            per_endpoint: false,
            track_latency: false,
            warmup_requests: 0,
            expected_hit_rate: None,
        }
    }
}

impl CacheMetricsConfig {
    /// Returns the default value for the `enabled` field.
    ///
    /// Disabled by default to avoid overhead in existing scenarios.
    #[must_use]
    pub const fn default_enabled() -> bool {
        false
    }

    /// Creates a configuration for cold cache testing.
    ///
    /// Configured to expect 0% cache hit rate with all tracking enabled.
    #[must_use]
    pub const fn cold_cache() -> Self {
        Self {
            enabled: true,
            per_endpoint: true,
            track_latency: true,
            warmup_requests: 0,
            expected_hit_rate: Some(0.0),
        }
    }

    /// Creates a configuration for warm cache testing.
    ///
    /// Configured with specified warmup requests and 80% expected hit rate.
    #[must_use]
    pub const fn warm_cache(warmup: u32) -> Self {
        Self {
            enabled: true,
            per_endpoint: true,
            track_latency: true,
            warmup_requests: warmup,
            expected_hit_rate: Some(0.8),
        }
    }

    /// Returns true if cache metrics tracking is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// =============================================================================
// Error Configuration
// =============================================================================

/// Error handling and timeout configuration for benchmark scenarios.
///
/// Provides fine-grained control over request timeouts, retry behavior,
/// error rate thresholds, and chaos testing support.
///
/// # Example
///
/// ```yaml
/// error_config:
///   timeout_ms: 5000
///   connect_timeout_ms: 1000
///   max_retries: 2
///   retry_delay_ms: 500
///   expected_error_rate: 0.05
///   fail_on_error_threshold: true
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorConfig {
    /// Request timeout in milliseconds.
    #[serde(default = "ErrorConfig::default_timeout_ms")]
    pub timeout_ms: u64,

    /// Connection timeout in milliseconds.
    #[serde(default = "ErrorConfig::default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,

    /// Maximum retry attempts.
    #[serde(default)]
    pub max_retries: u32,

    /// Retry delay in milliseconds.
    #[serde(default = "ErrorConfig::default_retry_delay_ms")]
    pub retry_delay_ms: u64,

    /// Expected error rate threshold (0.0-1.0).
    #[serde(default)]
    pub expected_error_rate: Option<f64>,

    /// Fail test if error rate exceeds threshold.
    #[serde(default)]
    pub fail_on_error_threshold: bool,

    /// Inject errors for chaos testing (rate: 0.0-1.0).
    #[serde(default)]
    pub inject_error_rate: Option<f64>,
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Self::default_timeout_ms(),
            connect_timeout_ms: Self::default_connect_timeout_ms(),
            max_retries: 0,
            retry_delay_ms: Self::default_retry_delay_ms(),
            expected_error_rate: None,
            fail_on_error_threshold: false,
            inject_error_rate: None,
        }
    }
}

impl ErrorConfig {
    /// Returns the default request timeout (30 seconds).
    #[must_use]
    pub const fn default_timeout_ms() -> u64 {
        30000
    }

    /// Returns the default connection timeout (5 seconds).
    #[must_use]
    pub const fn default_connect_timeout_ms() -> u64 {
        5000
    }

    /// Returns the default retry delay (1 second).
    #[must_use]
    pub const fn default_retry_delay_ms() -> u64 {
        1000
    }

    /// Creates a configuration optimized for stress testing.
    ///
    /// Uses shorter timeouts and allows 10% error rate.
    #[must_use]
    pub const fn stress_test() -> Self {
        Self {
            timeout_ms: 5000,
            connect_timeout_ms: 1000,
            max_retries: 0,
            retry_delay_ms: 100,
            expected_error_rate: Some(0.1),
            fail_on_error_threshold: true,
            inject_error_rate: None,
        }
    }

    /// Creates a configuration for chaos testing with error injection.
    ///
    /// Configures retry behavior and sets expected error rate to 1.5x the injection rate.
    #[must_use]
    pub fn chaos_test(inject_rate: f64) -> Self {
        Self {
            timeout_ms: 10000,
            connect_timeout_ms: 2000,
            max_retries: 2,
            retry_delay_ms: 500,
            expected_error_rate: Some(inject_rate * 1.5),
            fail_on_error_threshold: true,
            inject_error_rate: Some(inject_rate),
        }
    }

    /// Returns true if any error handling configuration is active.
    #[must_use]
    pub const fn is_configured(&self) -> bool {
        self.max_retries > 0
            || self.expected_error_rate.is_some()
            || self.inject_error_rate.is_some()
    }
}

// =============================================================================
// Concurrency Configuration
// =============================================================================

/// Concurrency configuration for stress testing.
///
/// Allows fine-grained control over worker threads, pool sizes, and connection limits
/// to test backpressure behavior and resource contention scenarios.
///
/// # Example
///
/// ```yaml
/// concurrency:
///   worker_threads: 4
///   database_pool_size: 8
///   redis_pool_size: 8
///   max_connections: 100
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConcurrencyConfig {
    /// Number of Axum worker threads.
    pub worker_threads: u32,
    /// Database connection pool size.
    pub database_pool_size: u32,
    /// Redis connection pool size.
    pub redis_pool_size: u32,
    /// Maximum number of simultaneous connections.
    pub max_connections: u32,
}

const fn default_max_connections() -> u32 {
    100
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            worker_threads: default_worker_threads(),
            database_pool_size: default_db_pool_size(),
            redis_pool_size: default_redis_pool_size(),
            max_connections: default_max_connections(),
        }
    }
}

impl ConcurrencyConfig {
    /// Creates a small pool configuration for stress testing.
    ///
    /// Pool sizes: 4, Workers: 1
    #[must_use]
    pub const fn small_pool() -> Self {
        Self {
            worker_threads: 1,
            database_pool_size: 4,
            redis_pool_size: 4,
            max_connections: 50,
        }
    }

    /// Creates a medium pool configuration.
    ///
    /// Pool sizes: 8, Workers: 4
    #[must_use]
    pub const fn medium_pool() -> Self {
        Self {
            worker_threads: 4,
            database_pool_size: 8,
            redis_pool_size: 8,
            max_connections: 100,
        }
    }

    /// Creates a large pool configuration.
    ///
    /// Pool sizes: 32, Workers: 8
    #[must_use]
    pub const fn large_pool() -> Self {
        Self {
            worker_threads: 8,
            database_pool_size: 32,
            redis_pool_size: 32,
            max_connections: 200,
        }
    }

    /// Returns a human-readable description of this configuration.
    #[must_use]
    pub fn description(&self) -> String {
        format!(
            "workers={}, db_pool={}, redis_pool={}, max_conn={}",
            self.worker_threads,
            self.database_pool_size,
            self.redis_pool_size,
            self.max_connections
        )
    }

    /// Returns the preset name based on the configuration values.
    ///
    /// Returns "small", "medium", "large", or "custom" based on matching
    /// preset configurations.
    #[must_use]
    pub const fn preset_name(&self) -> &'static str {
        if self.matches_preset(&Self::small_pool()) {
            "small"
        } else if self.matches_preset(&Self::medium_pool()) {
            "medium"
        } else if self.matches_preset(&Self::large_pool()) {
            "large"
        } else {
            "custom"
        }
    }

    /// Checks if this configuration matches another configuration.
    const fn matches_preset(&self, other: &Self) -> bool {
        self.worker_threads == other.worker_threads
            && self.database_pool_size == other.database_pool_size
            && self.redis_pool_size == other.redis_pool_size
            && self.max_connections == other.max_connections
    }
}

// =============================================================================
// Contention Level
// =============================================================================

/// Contention level for write-heavy scenarios.
///
/// Defines the degree of resource contention in benchmark scenarios,
/// particularly useful for testing persistent data structures and
/// database write performance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentionLevel {
    /// Low contention (read-heavy, minimal resource conflicts).
    ///
    /// - Read ratio: ~90%
    /// - Accesses distributed across many resources
    #[default]
    Low,
    /// Medium contention (balanced read/write).
    ///
    /// - Read ratio: ~50%
    /// - Some overlap in accessed resources
    Medium,
    /// High contention (write-heavy, concentrated resource access).
    ///
    /// - Read ratio: ~10%
    /// - Multiple threads targeting same resources
    High,
}

impl ContentionLevel {
    /// Returns the read operation ratio (0-100).
    #[must_use]
    pub const fn read_ratio(&self) -> u8 {
        match self {
            Self::Low => 90,
            Self::Medium => 50,
            Self::High => 10,
        }
    }

    /// Returns the write operation ratio (0-100).
    #[must_use]
    pub const fn write_ratio(&self) -> u8 {
        match self {
            Self::Low => 10,
            Self::Medium => 50,
            Self::High => 90,
        }
    }

    /// Returns the number of target resources (lower = more contention).
    ///
    /// High contention concentrates access on fewer resources.
    #[must_use]
    pub const fn target_resource_count(&self) -> usize {
        match self {
            Self::Low => 1000,
            Self::Medium => 100,
            Self::High => 10,
        }
    }

    /// Returns a human-readable description.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Low => "low contention (read-heavy, distributed)",
            Self::Medium => "medium contention (balanced)",
            Self::High => "high contention (write-heavy, concentrated)",
        }
    }
}

impl FromStr for ContentionLevel {
    type Err = ScenarioError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "low" | "read_heavy" | "distributed" => Ok(Self::Low),
            "medium" | "balanced" | "mixed" => Ok(Self::Medium),
            "high" | "write_heavy" | "concentrated" => Ok(Self::High),
            _ => Err(ScenarioError::InvalidContentionLevel(value.to_string())),
        }
    }
}

impl std::fmt::Display for ContentionLevel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(formatter, "low"),
            Self::Medium => write!(formatter, "medium"),
            Self::High => write!(formatter, "high"),
        }
    }
}

/// Worker configuration for the API server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Number of Axum worker threads.
    #[serde(default = "default_worker_threads")]
    pub worker_threads: u32,
}

const fn default_worker_threads() -> u32 {
    4
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            worker_threads: default_worker_threads(),
        }
    }
}

/// Performance thresholds for benchmark validation.
///
/// When deserializing from YAML, omitted fields use `Default::default()` values:
/// - `max_error_rate`: 0.01 (1%)
/// - `p99_latency_ms`: 100ms
/// - `min_rps_achieved`: 0 (no minimum)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Thresholds {
    /// Maximum acceptable error rate (0.0 - 1.0).
    pub max_error_rate: f64,
    /// Maximum acceptable p99 latency in milliseconds.
    pub p99_latency_ms: u64,
    /// Minimum RPS that must be achieved.
    pub min_rps_achieved: u32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_error_rate: 0.01,
            p99_latency_ms: 100,
            min_rps_achieved: 0,
        }
    }
}

/// Additional metadata for scenario analysis.
///
/// When deserializing from YAML, omitted fields use `Default::default()` values
/// (empty strings, 0 for numbers, empty vec for tags).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScenarioMetadata {
    /// Type of test (e.g., `payload_variation`, `stress_test`).
    #[serde(default)]
    pub test_type: String,
    /// Payload variant name.
    #[serde(default)]
    pub payload_variant: String,
    /// Estimated payload size in bytes.
    #[serde(default)]
    pub payload_size_bytes: u64,
    /// Number of tags in payload.
    #[serde(default)]
    pub tag_count: u32,
    /// Number of subtasks in payload.
    #[serde(default)]
    pub subtask_count: u32,
    /// Purpose of this test.
    #[serde(default)]
    pub purpose: String,
    /// Additional tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

// =============================================================================
// Scenario Builder
// =============================================================================

/// Builder for `BenchmarkScenario`.
#[derive(Debug, Clone, Default)]
pub struct BenchmarkScenarioBuilder {
    scenario: BenchmarkScenario,
}

impl BenchmarkScenarioBuilder {
    /// Creates a new builder with the given scenario name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            scenario: BenchmarkScenario {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Sets the scenario description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.scenario.description = description.into();
        self
    }

    /// Sets the storage mode.
    #[must_use]
    pub const fn storage_mode(mut self, mode: StorageMode) -> Self {
        self.scenario.storage_mode = mode;
        self
    }

    /// Sets the cache mode.
    #[must_use]
    pub const fn cache_mode(mut self, mode: CacheMode) -> Self {
        self.scenario.cache_mode = mode;
        self
    }

    /// Sets the load pattern.
    #[must_use]
    pub const fn load_pattern(mut self, pattern: LoadPattern) -> Self {
        self.scenario.load_pattern = pattern;
        self
    }

    /// Sets the cache state.
    #[must_use]
    pub const fn cache_state(mut self, state: CacheState) -> Self {
        self.scenario.cache_state = state;
        self
    }

    /// Sets the data scale.
    #[must_use]
    pub const fn data_scale(mut self, scale: DataScale) -> Self {
        self.scenario.data_scale = scale;
        self
    }

    /// Sets the extended data scale configuration.
    ///
    /// This provides fine-grained control over data seeding with custom
    /// record counts, random seeds, and incremental seeding options.
    #[must_use]
    pub const fn data_scale_config(mut self, config: DataScaleConfig) -> Self {
        self.scenario.data_scale_config = Some(config);
        self
    }

    /// Sets the payload variant.
    #[must_use]
    pub const fn payload_variant(mut self, variant: PayloadVariant) -> Self {
        self.scenario.payload_variant = variant;
        self
    }

    /// Sets the RPS profile.
    #[must_use]
    pub const fn rps_profile(mut self, profile: RpsProfile) -> Self {
        self.scenario.rps_profile = profile;
        self
    }

    /// Sets the test duration in seconds.
    #[must_use]
    pub const fn duration_seconds(mut self, seconds: u64) -> Self {
        self.scenario.duration_seconds = seconds;
        self
    }

    /// Sets the number of concurrent connections.
    #[must_use]
    pub const fn connections(mut self, connections: u32) -> Self {
        self.scenario.connections = connections;
        self
    }

    /// Sets the number of threads.
    #[must_use]
    pub const fn threads(mut self, threads: u32) -> Self {
        self.scenario.threads = threads;
        self
    }

    /// Sets the target RPS.
    #[must_use]
    pub const fn target_rps(mut self, rps: u32) -> Self {
        self.scenario.target_rps = rps;
        self
    }

    /// Sets the warmup duration in seconds.
    #[must_use]
    pub const fn warmup_seconds(mut self, seconds: u64) -> Self {
        self.scenario.warmup_seconds = seconds;
        self
    }

    /// Sets the target endpoints.
    #[must_use]
    pub fn endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.scenario.endpoints = endpoints;
        self
    }

    /// Sets the HTTP methods for the benchmark.
    #[must_use]
    pub fn http_methods(mut self, http_methods: Vec<String>) -> Self {
        self.scenario.http_methods = http_methods;
        self
    }

    /// Sets the pool configuration.
    #[must_use]
    pub const fn pool_sizes(mut self, pool_config: PoolConfig) -> Self {
        self.scenario.pool_sizes = Some(pool_config);
        self
    }

    /// Sets the worker configuration.
    #[must_use]
    pub const fn worker_config(mut self, worker_config: WorkerConfig) -> Self {
        self.scenario.worker_config = Some(worker_config);
        self
    }

    /// Sets the concurrency configuration.
    #[must_use]
    pub const fn concurrency(mut self, concurrency_config: ConcurrencyConfig) -> Self {
        self.scenario.concurrency = Some(concurrency_config);
        self
    }

    /// Sets the contention level.
    #[must_use]
    pub const fn contention_level(mut self, level: ContentionLevel) -> Self {
        self.scenario.contention_level = level;
        self
    }

    /// Sets the profiling configuration.
    #[must_use]
    pub fn profiling(mut self, profiling_config: ProfilingConfig) -> Self {
        self.scenario.profiling = profiling_config;
        self
    }

    /// Sets the cache metrics configuration.
    #[must_use]
    pub const fn cache_metrics(mut self, cache_metrics_config: CacheMetricsConfig) -> Self {
        self.scenario.cache_metrics = cache_metrics_config;
        self
    }

    /// Sets the error handling and timeout configuration.
    #[must_use]
    pub const fn error_config(mut self, error_config: ErrorConfig) -> Self {
        self.scenario.error_config = error_config;
        self
    }

    /// Sets the custom environment variables.
    ///
    /// These variables are exported before starting the API server,
    /// allowing scenario-specific configuration like cache settings.
    #[must_use]
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.scenario.environment = environment;
        self
    }

    /// Builds the scenario.
    #[must_use]
    pub fn build(self) -> BenchmarkScenario {
        self.scenario
    }
}

// =============================================================================
// Scenario Matrix
// =============================================================================

/// Generates all combinations of scenarios from a matrix specification.
///
/// Supports full combinatorial generation across all configuration dimensions:
/// storage modes, cache modes, load patterns, cache states, data scales,
/// payload variants, and RPS profiles.
///
/// **Important**: Uses `deny_unknown_fields` to catch typos in YAML field names.
/// Without this, typos in optional fields (like `data_scales`) would silently
/// fall back to defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioMatrix {
    /// Base name prefix for generated scenarios.
    pub name_prefix: String,
    /// Base description for generated scenarios.
    #[serde(default)]
    pub description_template: String,
    /// Storage modes to test.
    pub storage_modes: Vec<StorageMode>,
    /// Cache modes to test.
    pub cache_modes: Vec<CacheMode>,
    /// Load patterns to test.
    pub load_patterns: Vec<LoadPattern>,
    /// Cache states to test.
    pub cache_states: Vec<CacheState>,
    /// Data scales to test (optional, defaults to `\[Medium\]`).
    #[serde(default = "default_data_scales")]
    pub data_scales: Vec<DataScale>,
    /// Payload variants to test (optional, defaults to `\[Standard\]`).
    #[serde(default = "default_payload_variants")]
    pub payload_variants: Vec<PayloadVariant>,
    /// RPS profiles to test (optional, defaults to `\[Constant\]`).
    #[serde(default = "default_rps_profiles")]
    pub rps_profiles: Vec<RpsProfile>,
    /// Default duration for generated scenarios.
    #[serde(default = "default_duration_seconds")]
    pub default_duration_seconds: u64,
    /// Default connections for generated scenarios.
    #[serde(default = "default_connections")]
    pub default_connections: u32,
    /// Default threads for generated scenarios.
    #[serde(default = "default_threads")]
    pub default_threads: u32,
}

fn default_data_scales() -> Vec<DataScale> {
    vec![DataScale::Medium]
}

fn default_payload_variants() -> Vec<PayloadVariant> {
    vec![PayloadVariant::Standard]
}

fn default_rps_profiles() -> Vec<RpsProfile> {
    vec![RpsProfile::Constant]
}

impl ScenarioMatrix {
    /// Generates all scenario combinations.
    ///
    /// Total scenarios = storage\_modes x cache\_modes x load\_patterns
    ///                   x cache\_states x data\_scales x payload\_variants
    ///                   x rps\_profiles
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn generate_scenarios(&self) -> Vec<BenchmarkScenario> {
        let mut scenarios = Vec::new();

        for &storage_mode in &self.storage_modes {
            for &cache_mode in &self.cache_modes {
                for &load_pattern in &self.load_patterns {
                    for &cache_state in &self.cache_states {
                        for &data_scale in &self.data_scales {
                            for &payload_variant in &self.payload_variants {
                                for rps_profile in &self.rps_profiles {
                                    let name = format!(
                                        "{}_{storage_mode}_{cache_mode}_{load_pattern}_{cache_state}_{data_scale}_{payload_variant}_{rps_profile}",
                                        self.name_prefix
                                    );

                                    let description = if self.description_template.is_empty() {
                                        format!(
                                            "Generated scenario: {storage_mode} storage, {cache_mode} cache, {load_pattern} load, {cache_state} state, {data_scale} scale, {payload_variant} payload, {rps_profile} rps"
                                        )
                                    } else {
                                        self.description_template.clone()
                                    };

                                    scenarios.push(BenchmarkScenario {
                                        name,
                                        description,
                                        storage_mode,
                                        cache_mode,
                                        load_pattern,
                                        cache_state,
                                        data_scale,
                                        payload_variant,
                                        rps_profile: rps_profile.clone(),
                                        duration_seconds: self.default_duration_seconds,
                                        connections: self.default_connections,
                                        threads: self.default_threads,
                                        target_rps: 0,
                                        warmup_seconds: default_warmup_seconds(),
                                        endpoints: Vec::new(),
                                        http_methods: Vec::new(),
                                        pool_sizes: None,
                                        worker_config: None,
                                        thresholds: None,
                                        metadata: None,
                                        concurrency: None,
                                        contention_level: ContentionLevel::default(),
                                        profiling: ProfilingConfig::default(),
                                        data_scale_config: None,
                                        cache_metrics: CacheMetricsConfig::default(),
                                        error_config: ErrorConfig::default(),
                                        environment: HashMap::new(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        scenarios
    }

    /// Returns the total number of scenario combinations.
    #[must_use]
    pub const fn combination_count(&self) -> usize {
        self.storage_modes.len()
            * self.cache_modes.len()
            * self.load_patterns.len()
            * self.cache_states.len()
            * self.data_scales.len()
            * self.payload_variants.len()
            * self.rps_profiles.len()
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during scenario processing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScenarioError {
    /// Invalid load pattern value.
    #[error("Invalid load pattern: '{0}'. Expected 'read_heavy', 'write_heavy', or 'mixed'")]
    InvalidLoadPattern(String),

    /// Invalid cache state value.
    #[error("Invalid cache state: '{0}'. Expected 'cold' or 'warm'")]
    InvalidCacheState(String),

    /// Invalid data scale value.
    #[error("Invalid data scale: '{0}'. Expected 'small', 'medium', or 'large'")]
    InvalidDataScale(String),

    /// Invalid payload variant value.
    #[error(
        "Invalid payload variant: '{0}'. Expected 'minimal', 'standard', 'complex', or 'heavy'"
    )]
    InvalidPayloadVariant(String),

    /// Invalid RPS profile value.
    #[error(
        "Invalid RPS profile: '{0}'. Expected 'constant', 'ramp_up_down', 'burst', or 'step_up'"
    )]
    InvalidRpsProfile(String),

    /// Invalid contention level value.
    #[error("Invalid contention level: '{0}'. Expected 'low', 'medium', or 'high'")]
    InvalidContentionLevel(String),

    /// File read error.
    #[error("Failed to read scenario file: {0}")]
    FileRead(String),

    /// YAML parse error.
    #[error("Failed to parse scenario YAML: {0}")]
    YamlParse(String),

    /// YAML serialize error.
    #[error("Failed to serialize scenario to YAML: {0}")]
    YamlSerialize(String),

    /// Template not found.
    #[error("Template not found: '{0}'")]
    TemplateNotFound(String),
}

// =============================================================================
// Serde implementations for StorageMode and CacheMode
// =============================================================================

impl Serialize for StorageMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::InMemory => serializer.serialize_str("in_memory"),
            Self::Postgres => serializer.serialize_str("postgres"),
        }
    }
}

impl<'de> Deserialize<'de> for StorageMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for CacheMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::InMemory => serializer.serialize_str("in_memory"),
            Self::Redis => serializer.serialize_str("redis"),
        }
    }
}

impl<'de> Deserialize<'de> for CacheMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for StorageMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InMemory => write!(formatter, "in_memory"),
            Self::Postgres => write!(formatter, "postgres"),
        }
    }
}

impl std::fmt::Display for CacheMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InMemory => write!(formatter, "in_memory"),
            Self::Redis => write!(formatter, "redis"),
        }
    }
}

// =============================================================================
// Partial Scenario
// =============================================================================

/// Partial scenario configuration for templates and inheritance.
///
/// All fields are `Option<T>` to allow selective overriding of specific
/// configuration values while inheriting others from a template.
///
/// # Example
///
/// ```rust
/// use task_management_benchmark_api::infrastructure::{
///     PartialScenario, StorageMode, CacheMode, LoadPattern,
/// };
///
/// let partial = PartialScenario {
///     storage_mode: Some(StorageMode::Postgres),
///     cache_mode: Some(CacheMode::Redis),
///     load_pattern: Some(LoadPattern::Mixed),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialScenario {
    /// Storage mode override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_mode: Option<StorageMode>,

    /// Cache mode override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_mode: Option<CacheMode>,

    /// Load pattern override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_pattern: Option<LoadPattern>,

    /// Cache state override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_state: Option<CacheState>,

    /// Data scale override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_scale: Option<DataScale>,

    /// Payload variant override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_variant: Option<PayloadVariant>,

    /// RPS profile override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rps_profile: Option<RpsProfile>,

    /// Contention level override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contention_level: Option<ContentionLevel>,

    /// Duration in seconds override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,

    /// Number of connections override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<u32>,

    /// Number of threads override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threads: Option<u32>,

    /// Warmup duration in seconds override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmup_seconds: Option<u64>,

    /// Target RPS override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_rps: Option<u32>,

    /// Profiling configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiling: Option<ProfilingConfig>,

    /// Extended data scale configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_scale_config: Option<DataScaleConfig>,

    /// Cache metrics configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_metrics: Option<CacheMetricsConfig>,

    /// Error handling and timeout configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_config: Option<ErrorConfig>,

    /// Concurrency configuration override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<ConcurrencyConfig>,

    /// Performance thresholds override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thresholds: Option<Thresholds>,

    /// Custom environment variables override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
}

impl PartialScenario {
    /// Merges this partial scenario with another.
    ///
    /// The `other` partial scenario takes precedence - any `Some` values in
    /// `other` will override values in `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     PartialScenario, StorageMode, CacheMode,
    /// };
    ///
    /// let base = PartialScenario {
    ///     storage_mode: Some(StorageMode::InMemory),
    ///     cache_mode: Some(CacheMode::InMemory),
    ///     ..Default::default()
    /// };
    ///
    /// let override_partial = PartialScenario {
    ///     storage_mode: Some(StorageMode::Postgres),
    ///     ..Default::default()
    /// };
    ///
    /// let merged = base.merge(&override_partial);
    /// assert_eq!(merged.storage_mode, Some(StorageMode::Postgres));
    /// assert_eq!(merged.cache_mode, Some(CacheMode::InMemory));
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            storage_mode: other.storage_mode.or(self.storage_mode),
            cache_mode: other.cache_mode.or(self.cache_mode),
            load_pattern: other.load_pattern.or(self.load_pattern),
            cache_state: other.cache_state.or(self.cache_state),
            data_scale: other.data_scale.or(self.data_scale),
            payload_variant: other.payload_variant.or(self.payload_variant),
            rps_profile: other
                .rps_profile
                .clone()
                .or_else(|| self.rps_profile.clone()),
            contention_level: other.contention_level.or(self.contention_level),
            duration_seconds: other.duration_seconds.or(self.duration_seconds),
            connections: other.connections.or(self.connections),
            threads: other.threads.or(self.threads),
            warmup_seconds: other.warmup_seconds.or(self.warmup_seconds),
            target_rps: other.target_rps.or(self.target_rps),
            profiling: other.profiling.clone().or_else(|| self.profiling.clone()),
            data_scale_config: other
                .data_scale_config
                .clone()
                .or_else(|| self.data_scale_config.clone()),
            cache_metrics: other
                .cache_metrics
                .clone()
                .or_else(|| self.cache_metrics.clone()),
            error_config: other
                .error_config
                .clone()
                .or_else(|| self.error_config.clone()),
            concurrency: other
                .concurrency
                .clone()
                .or_else(|| self.concurrency.clone()),
            thresholds: other.thresholds.clone().or_else(|| self.thresholds.clone()),
            environment: other
                .environment
                .clone()
                .or_else(|| self.environment.clone()),
        }
    }

    /// Applies this partial scenario's values to a full `BenchmarkScenario`.
    ///
    /// Only fields with `Some` values are applied; `None` fields are skipped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, StorageMode, CacheMode, LoadPattern,
    ///     CacheState, DataScale, PayloadVariant, RpsProfile,
    /// };
    /// use task_management_benchmark_api::infrastructure::scenario::PartialScenario;
    ///
    /// let mut scenario = BenchmarkScenario::builder("test")
    ///     .description("Test scenario")
    ///     .storage_mode(StorageMode::InMemory)
    ///     .cache_mode(CacheMode::InMemory)
    ///     .load_pattern(LoadPattern::ReadHeavy)
    ///     .cache_state(CacheState::Warm)
    ///     .data_scale(DataScale::Medium)
    ///     .payload_variant(PayloadVariant::Standard)
    ///     .rps_profile(RpsProfile::Constant)
    ///     .build();
    ///
    /// let partial = PartialScenario {
    ///     storage_mode: Some(StorageMode::Postgres),
    ///     duration_seconds: Some(120),
    ///     ..Default::default()
    /// };
    ///
    /// partial.apply_to(&mut scenario);
    ///
    /// assert_eq!(scenario.storage_mode, StorageMode::Postgres);
    /// assert_eq!(scenario.duration_seconds, 120);
    /// assert_eq!(scenario.cache_mode, CacheMode::InMemory); // Unchanged
    /// ```
    pub fn apply_to(&self, scenario: &mut BenchmarkScenario) {
        if let Some(value) = self.storage_mode {
            scenario.storage_mode = value;
        }
        if let Some(value) = self.cache_mode {
            scenario.cache_mode = value;
        }
        if let Some(value) = self.load_pattern {
            scenario.load_pattern = value;
        }
        if let Some(value) = self.cache_state {
            scenario.cache_state = value;
        }
        if let Some(value) = self.data_scale {
            scenario.data_scale = value;
        }
        if let Some(value) = self.payload_variant {
            scenario.payload_variant = value;
        }
        if let Some(ref value) = self.rps_profile {
            scenario.rps_profile = value.clone();
        }
        if let Some(value) = self.contention_level {
            scenario.contention_level = value;
        }
        if let Some(value) = self.duration_seconds {
            scenario.duration_seconds = value;
        }
        if let Some(value) = self.connections {
            scenario.connections = value;
        }
        if let Some(value) = self.threads {
            scenario.threads = value;
        }
        if let Some(value) = self.warmup_seconds {
            scenario.warmup_seconds = value;
        }
        if let Some(value) = self.target_rps {
            scenario.target_rps = value;
        }
        if let Some(ref value) = self.profiling {
            scenario.profiling = value.clone();
        }
        if let Some(ref value) = self.data_scale_config {
            scenario.data_scale_config = Some(value.clone());
        }
        if let Some(ref value) = self.cache_metrics {
            scenario.cache_metrics = value.clone();
        }
        if let Some(ref value) = self.error_config {
            scenario.error_config = value.clone();
        }
        if let Some(ref value) = self.concurrency {
            scenario.concurrency = Some(value.clone());
        }
        if let Some(ref value) = self.thresholds {
            scenario.thresholds = Some(value.clone());
        }
        if let Some(ref value) = self.environment {
            scenario.environment.clone_from(value);
        }
    }
}

impl From<&BenchmarkScenario> for PartialScenario {
    /// Creates a `PartialScenario` from a `BenchmarkScenario`.
    ///
    /// All fields from the `BenchmarkScenario` are converted to `Some` values
    /// in the `PartialScenario`. This is useful for template inheritance where
    /// a scenario's values should override template defaults.
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, StorageMode, CacheMode, LoadPattern, CacheState,
    ///     DataScale, PayloadVariant, RpsProfile,
    /// };
    /// use task_management_benchmark_api::infrastructure::scenario::PartialScenario;
    ///
    /// let scenario = BenchmarkScenario::builder("test")
    ///     .description("Test")
    ///     .storage_mode(StorageMode::Postgres)
    ///     .cache_mode(CacheMode::Redis)
    ///     .load_pattern(LoadPattern::Mixed)
    ///     .cache_state(CacheState::Warm)
    ///     .data_scale(DataScale::Large)
    ///     .payload_variant(PayloadVariant::Complex)
    ///     .rps_profile(RpsProfile::Burst)
    ///     .build();
    ///
    /// let partial = PartialScenario::from(&scenario);
    ///
    /// assert_eq!(partial.storage_mode, Some(StorageMode::Postgres));
    /// assert_eq!(partial.cache_mode, Some(CacheMode::Redis));
    /// assert_eq!(partial.load_pattern, Some(LoadPattern::Mixed));
    /// ```
    fn from(scenario: &BenchmarkScenario) -> Self {
        Self {
            storage_mode: Some(scenario.storage_mode),
            cache_mode: Some(scenario.cache_mode),
            load_pattern: Some(scenario.load_pattern),
            cache_state: Some(scenario.cache_state),
            data_scale: Some(scenario.data_scale),
            payload_variant: Some(scenario.payload_variant),
            rps_profile: Some(scenario.rps_profile.clone()),
            contention_level: Some(scenario.contention_level),
            duration_seconds: Some(scenario.duration_seconds),
            connections: Some(scenario.connections),
            threads: Some(scenario.threads),
            warmup_seconds: Some(scenario.warmup_seconds),
            target_rps: Some(scenario.target_rps),
            profiling: Some(scenario.profiling.clone()),
            data_scale_config: scenario.data_scale_config.clone(),
            cache_metrics: Some(scenario.cache_metrics.clone()),
            error_config: Some(scenario.error_config.clone()),
            concurrency: scenario.concurrency.clone(),
            thresholds: scenario.thresholds.clone(),
            environment: if scenario.environment.is_empty() {
                None
            } else {
                Some(scenario.environment.clone())
            },
        }
    }
}

// =============================================================================
// Scenario Template
// =============================================================================

/// Template for creating benchmark scenarios with predefined defaults.
///
/// Templates allow defining reusable scenario configurations that can be
/// extended and customized by individual scenarios.
///
/// # Example
///
/// ```yaml
/// name: "high_load"
/// description: "Template for high load scenarios"
///
/// storage_mode: postgres
/// cache_mode: redis
/// load_pattern: mixed
/// cache_state: warm
/// data_scale: large
/// contention_level: high
///
/// duration_seconds: 120
/// connections: 100
/// threads: 16
/// target_rps: 5000
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTemplate {
    /// Template name.
    pub name: String,

    /// Template description.
    pub description: String,

    /// Base scenario configuration (partial).
    #[serde(flatten)]
    pub base: PartialScenario,
}

impl ScenarioTemplate {
    /// Creates a new scenario template with the given name and description.
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            base: PartialScenario::default(),
        }
    }

    /// Loads a template from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ScenarioError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| ScenarioError::FileRead(error.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parses a template from YAML content.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the YAML is invalid.
    pub fn from_yaml(content: &str) -> Result<Self, ScenarioError> {
        serde_yaml::from_str(content).map_err(|error| ScenarioError::YamlParse(error.to_string()))
    }

    /// Serializes the template to YAML.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if serialization fails.
    pub fn to_yaml(&self) -> Result<String, ScenarioError> {
        serde_yaml::to_string(self).map_err(|error| ScenarioError::YamlSerialize(error.to_string()))
    }
}

// =============================================================================
// Scenario Validation
// =============================================================================

/// Validation result for a benchmark scenario.
///
/// Contains the overall validity status along with any errors and warnings
/// detected during validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScenarioValidation {
    /// Whether the scenario is valid (no errors).
    pub is_valid: bool,
    /// List of validation errors (blocking issues).
    pub errors: Vec<String>,
    /// List of validation warnings (non-blocking recommendations).
    pub warnings: Vec<String>,
}

impl ScenarioValidation {
    /// Creates a new empty validation result (valid by default).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Adds a validation error.
    pub fn add_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
        self.is_valid = false;
    }

    /// Adds a validation warning.
    pub fn add_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
}

impl BenchmarkScenario {
    /// Validates the scenario configuration.
    ///
    /// Performs validation checks including:
    /// - Required field validation (name, description)
    /// - Logical consistency checks (threads vs connections)
    /// - Cache state consistency (cold cache + warmup requests)
    /// - Data scale consistency (large data + short duration)
    /// - Error configuration consistency
    ///
    /// # Example
    ///
    /// ```rust
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, StorageMode, CacheMode, LoadPattern, CacheState,
    ///     DataScale, PayloadVariant, RpsProfile, CacheMetricsConfig,
    /// };
    ///
    /// let scenario = BenchmarkScenario::builder("test")
    ///     .description("A valid test scenario")
    ///     .storage_mode(StorageMode::InMemory)
    ///     .cache_mode(CacheMode::InMemory)
    ///     .load_pattern(LoadPattern::ReadHeavy)
    ///     .cache_state(CacheState::Warm)
    ///     .data_scale(DataScale::Medium)
    ///     .payload_variant(PayloadVariant::Standard)
    ///     .rps_profile(RpsProfile::Constant)
    ///     .build();
    ///
    /// let validation = scenario.validate();
    /// assert!(validation.is_valid);
    /// ```
    #[must_use]
    pub fn validate(&self) -> ScenarioValidation {
        let mut validation = ScenarioValidation::new();

        // Required field validation
        if self.name.is_empty() {
            validation.add_error("Scenario name is required");
        }

        if self.description.is_empty() {
            validation.add_error("Scenario description is required");
        }

        // Logical consistency checks
        if self.connections > 0 && self.threads > self.connections {
            validation.add_warning("threads > connections may cause issues");
        }

        // Cache state consistency
        if self.cache_state == CacheState::Cold && self.cache_metrics.warmup_requests > 0 {
            validation.add_warning("warmup_requests > 0 with cold cache state");
        }

        // Data scale consistency
        if self.data_scale == DataScale::Large && self.duration_seconds < 60 {
            validation.add_warning("Large data scale with short duration may not be meaningful");
        }

        // Error config consistency
        if self.error_config.inject_error_rate.is_some()
            && self.error_config.expected_error_rate.is_none()
        {
            validation.add_warning("Error injection enabled but no expected_error_rate set");
        }

        // Rate validations (0.0-1.0 range)
        if let Some(rate) = self.error_config.expected_error_rate
            && !(0.0..=1.0).contains(&rate)
        {
            validation.add_error(format!(
                "expected_error_rate must be between 0.0 and 1.0, got {rate}"
            ));
        }

        if let Some(rate) = self.error_config.inject_error_rate
            && !(0.0..=1.0).contains(&rate)
        {
            validation.add_error(format!(
                "inject_error_rate must be between 0.0 and 1.0, got {rate}"
            ));
        }

        if let Some(rate) = self.cache_metrics.expected_hit_rate
            && !(0.0..=1.0).contains(&rate)
        {
            validation.add_error(format!(
                "expected_hit_rate must be between 0.0 and 1.0, got {rate}"
            ));
        }

        // Duration and warmup relationship
        if self.warmup_seconds >= self.duration_seconds {
            validation.add_error("warmup_seconds must be less than duration_seconds");
        }

        // Connections and target_rps relationship
        if self.target_rps > 0 && self.connections > 0 {
            let rps_per_connection = self.target_rps / self.connections;
            if rps_per_connection > 1000 {
                validation.add_warning(format!(
                    "High RPS per connection ({} RPS / {} connections = {rps_per_connection} RPS/connection)",
                    self.target_rps, self.connections
                ));
            }
        }

        // Max connections validation
        if let Some(ref concurrency_config) = self.concurrency
            && concurrency_config.max_connections < self.connections
        {
            validation.add_warning(format!(
                "max_connections ({}) < connections ({})",
                concurrency_config.max_connections, self.connections
            ));
        }

        validation
    }
}

// =============================================================================
// Extendable Scenario
// =============================================================================

/// Scenario that can extend a template.
///
/// Allows scenarios to inherit configuration from templates while
/// overriding specific settings.
///
/// # Example
///
/// ```yaml
/// name: "production_load"
/// description: "Production-like load test extending high_load template"
/// extends: "high_load"
///
/// payload_variant: complex
/// rps_profile: burst
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendableScenario {
    /// Template to extend (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// Scenario configuration.
    #[serde(flatten)]
    pub scenario: BenchmarkScenario,
}

impl ExtendableScenario {
    /// Resolves the scenario by applying template inheritance.
    ///
    /// If `extends` is specified, the template's base configuration is used as
    /// the foundation, then the scenario's own values override the template defaults.
    ///
    /// The inheritance order is:
    /// 1. Start with default `BenchmarkScenario`
    /// 2. Apply template's base configuration
    /// 3. Apply scenario's specific values (overriding template values)
    ///
    /// **Note:** Since `ExtendableScenario` contains a full `BenchmarkScenario`,
    /// all scenario values (including those not explicitly set by the builder)
    /// will override the template's base configuration. This means the scenario's
    /// default values will override template values for fields not explicitly
    /// specified.
    ///
    /// # Errors
    ///
    /// Returns an error if the specified template is not found in the registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use task_management_benchmark_api::infrastructure::{
    ///     BenchmarkScenario, StorageMode, CacheMode, LoadPattern, CacheState,
    ///     DataScale, PayloadVariant, RpsProfile,
    /// };
    /// use task_management_benchmark_api::infrastructure::scenario::{
    ///     ExtendableScenario, ScenarioTemplate, PartialScenario,
    /// };
    ///
    /// // Create a template with base configuration
    /// let template = ScenarioTemplate {
    ///     name: "high_load".to_string(),
    ///     description: "High load template".to_string(),
    ///     base: PartialScenario {
    ///         storage_mode: Some(StorageMode::Postgres),
    ///         cache_mode: Some(CacheMode::Redis),
    ///         duration_seconds: Some(120),
    ///         connections: Some(100),
    ///         threads: Some(16),
    ///         ..Default::default()
    ///     },
    /// };
    ///
    /// let mut templates = HashMap::new();
    /// templates.insert("high_load".to_string(), template);
    ///
    /// // Create an extendable scenario that overrides template values
    /// // Note: All scenario values override template values, including defaults
    /// let extendable = ExtendableScenario {
    ///     extends: Some("high_load".to_string()),
    ///     scenario: BenchmarkScenario::builder("production_test")
    ///         .description("Production test")
    ///         .storage_mode(StorageMode::InMemory)
    ///         .cache_mode(CacheMode::InMemory)
    ///         .load_pattern(LoadPattern::Mixed)
    ///         .cache_state(CacheState::Warm)
    ///         .data_scale(DataScale::Large)
    ///         .payload_variant(PayloadVariant::Complex)
    ///         .rps_profile(RpsProfile::Burst)
    ///         .duration_seconds(120)  // Explicitly set to match template
    ///         .connections(100)       // Explicitly set to match template
    ///         .threads(16)            // Explicitly set to match template
    ///         .build(),
    /// };
    ///
    /// let resolved = extendable.resolve(&templates).unwrap();
    ///
    /// // Scenario values override template values
    /// assert_eq!(resolved.storage_mode, StorageMode::InMemory);
    /// assert_eq!(resolved.cache_mode, CacheMode::InMemory);
    /// assert_eq!(resolved.duration_seconds, 120);
    /// assert_eq!(resolved.connections, 100);
    /// assert_eq!(resolved.threads, 16);
    /// // Scenario's own values are preserved
    /// assert_eq!(resolved.load_pattern, LoadPattern::Mixed);
    /// assert_eq!(resolved.data_scale, DataScale::Large);
    /// ```
    pub fn resolve(
        &self,
        templates: &std::collections::HashMap<String, ScenarioTemplate>,
    ) -> Result<BenchmarkScenario, ScenarioError> {
        if let Some(ref template_name) = self.extends {
            let template = templates
                .get(template_name)
                .ok_or_else(|| ScenarioError::TemplateNotFound(template_name.clone()))?;

            // Start with default scenario
            let mut resolved = BenchmarkScenario::default();

            // 1. Apply template's base configuration
            template.base.apply_to(&mut resolved);

            // 2. Apply scenario's values (overriding template)
            let scenario_partial = PartialScenario::from(&self.scenario);
            scenario_partial.apply_to(&mut resolved);

            // Preserve the scenario's name and description
            resolved.name.clone_from(&self.scenario.name);
            resolved.description.clone_from(&self.scenario.description);

            Ok(resolved)
        } else {
            // No template, return scenario as-is
            Ok(self.scenario.clone())
        }
    }

    /// Loads an extendable scenario from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ScenarioError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|error| ScenarioError::FileRead(error.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parses an extendable scenario from YAML content.
    ///
    /// # Errors
    ///
    /// Returns `ScenarioError` if the YAML is invalid.
    pub fn from_yaml(content: &str) -> Result<Self, ScenarioError> {
        serde_yaml::from_str(content).map_err(|error| ScenarioError::YamlParse(error.to_string()))
    }
}

// =============================================================================
// Scenario Registry
// =============================================================================

/// Registry of available templates and scenarios.
///
/// Provides a central repository for managing scenario templates and
/// resolved scenarios.
#[derive(Debug, Default)]
pub struct ScenarioRegistry {
    templates: std::collections::HashMap<String, ScenarioTemplate>,
    scenarios: std::collections::HashMap<String, BenchmarkScenario>,
}

impl ScenarioRegistry {
    /// Creates a new empty scenario registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads templates from a directory.
    ///
    /// Scans the directory for `.yaml` and `.yml` files and parses them
    /// as scenario templates.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or if any template
    /// file is invalid.
    pub fn load_templates_from_directory(&mut self, directory: &Path) -> Result<(), ScenarioError> {
        let entries =
            fs::read_dir(directory).map_err(|error| ScenarioError::FileRead(error.to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|error| ScenarioError::FileRead(error.to_string()))?;
            let path = entry.path();

            if let Some(extension) = path.extension()
                && (extension == "yaml" || extension == "yml")
            {
                let template = ScenarioTemplate::from_file(&path)?;
                self.templates.insert(template.name.clone(), template);
            }
        }

        Ok(())
    }

    /// Registers a template.
    pub fn register_template(&mut self, template: ScenarioTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Gets a template by name.
    #[must_use]
    pub fn get_template(&self, name: &str) -> Option<&ScenarioTemplate> {
        self.templates.get(name)
    }

    /// Lists all template names.
    #[must_use]
    pub fn list_templates(&self) -> Vec<&str> {
        self.templates.keys().map(String::as_str).collect()
    }

    /// Returns all templates as a reference to the internal map.
    #[must_use]
    pub const fn templates(&self) -> &std::collections::HashMap<String, ScenarioTemplate> {
        &self.templates
    }

    /// Registers a resolved scenario.
    pub fn register_scenario(&mut self, scenario: BenchmarkScenario) {
        self.scenarios.insert(scenario.name.clone(), scenario);
    }

    /// Gets a scenario by name.
    #[must_use]
    pub fn get_scenario(&self, name: &str) -> Option<&BenchmarkScenario> {
        self.scenarios.get(name)
    }

    /// Lists all scenario names.
    #[must_use]
    pub fn list_scenarios(&self) -> Vec<&str> {
        self.scenarios.keys().map(String::as_str).collect()
    }

    /// Resolves an extendable scenario using the registry's templates.
    ///
    /// # Errors
    ///
    /// Returns an error if the template specified in `extends` is not found.
    pub fn resolve_scenario(
        &self,
        extendable: &ExtendableScenario,
    ) -> Result<BenchmarkScenario, ScenarioError> {
        extendable.resolve(&self.templates)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // LoadPattern Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("read_heavy", LoadPattern::ReadHeavy)]
    #[case("readheavy", LoadPattern::ReadHeavy)]
    #[case("read-heavy", LoadPattern::ReadHeavy)]
    #[case("write_heavy", LoadPattern::WriteHeavy)]
    #[case("writeheavy", LoadPattern::WriteHeavy)]
    #[case("write-heavy", LoadPattern::WriteHeavy)]
    #[case("mixed", LoadPattern::Mixed)]
    #[case("balanced", LoadPattern::Mixed)]
    fn test_load_pattern_from_str_valid(#[case] input: &str, #[case] expected: LoadPattern) {
        let result: Result<LoadPattern, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("")]
    #[case("light")]
    fn test_load_pattern_from_str_invalid(#[case] input: &str) {
        let result: Result<LoadPattern, _> = input.parse();
        assert!(result.is_err());
    }

    #[rstest]
    fn test_load_pattern_ratios() {
        assert_eq!(LoadPattern::ReadHeavy.read_ratio(), 90);
        assert_eq!(LoadPattern::ReadHeavy.write_ratio(), 10);
        assert_eq!(LoadPattern::WriteHeavy.read_ratio(), 10);
        assert_eq!(LoadPattern::WriteHeavy.write_ratio(), 90);
        assert_eq!(LoadPattern::Mixed.read_ratio(), 50);
        assert_eq!(LoadPattern::Mixed.write_ratio(), 50);
    }

    #[rstest]
    fn test_load_pattern_default() {
        assert_eq!(LoadPattern::default(), LoadPattern::ReadHeavy);
    }

    #[rstest]
    fn test_load_pattern_display() {
        assert_eq!(LoadPattern::ReadHeavy.to_string(), "read_heavy");
        assert_eq!(LoadPattern::WriteHeavy.to_string(), "write_heavy");
        assert_eq!(LoadPattern::Mixed.to_string(), "mixed");
    }

    // -------------------------------------------------------------------------
    // CacheState Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("cold", CacheState::Cold)]
    #[case("miss", CacheState::Cold)]
    #[case("empty", CacheState::Cold)]
    #[case("warm", CacheState::Warm)]
    #[case("hit", CacheState::Warm)]
    #[case("populated", CacheState::Warm)]
    fn test_cache_state_from_str_valid(#[case] input: &str, #[case] expected: CacheState) {
        let result: Result<CacheState, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("")]
    fn test_cache_state_from_str_invalid(#[case] input: &str) {
        let result: Result<CacheState, _> = input.parse();
        assert!(result.is_err());
    }

    #[rstest]
    fn test_cache_state_default() {
        assert_eq!(CacheState::default(), CacheState::Warm);
    }

    #[rstest]
    fn test_cache_state_display() {
        assert_eq!(CacheState::Cold.to_string(), "cold");
        assert_eq!(CacheState::Warm.to_string(), "warm");
    }

    // -------------------------------------------------------------------------
    // DataScale Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("small", DataScale::Small)]
    #[case("s", DataScale::Small)]
    #[case("1e2", DataScale::Small)]
    #[case("medium", DataScale::Medium)]
    #[case("m", DataScale::Medium)]
    #[case("1e4", DataScale::Medium)]
    #[case("large", DataScale::Large)]
    #[case("l", DataScale::Large)]
    #[case("1e6", DataScale::Large)]
    fn test_data_scale_from_str_valid(#[case] input: &str, #[case] expected: DataScale) {
        let result: Result<DataScale, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    fn test_data_scale_record_count() {
        assert_eq!(DataScale::Small.record_count(), 100);
        assert_eq!(DataScale::Medium.record_count(), 10_000);
        assert_eq!(DataScale::Large.record_count(), 1_000_000);
    }

    #[rstest]
    fn test_data_scale_default() {
        assert_eq!(DataScale::default(), DataScale::Medium);
    }

    #[rstest]
    fn test_data_scale_default_record_count() {
        assert_eq!(DataScale::Small.default_record_count(), 1_000);
        assert_eq!(DataScale::Medium.default_record_count(), 10_000);
        assert_eq!(DataScale::Large.default_record_count(), 1_000_000);
    }

    // -------------------------------------------------------------------------
    // DataScaleConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_data_scale_config_new() {
        let config = DataScaleConfig::new(DataScale::Large);
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, None);
        assert_eq!(config.seed, None);
        assert!(!config.incremental);
    }

    #[rstest]
    fn test_data_scale_config_with_record_count() {
        let config = DataScaleConfig::with_record_count(DataScale::Medium, 50_000);
        assert_eq!(config.scale, DataScale::Medium);
        assert_eq!(config.record_count, Some(50_000));
        assert_eq!(config.effective_record_count(), 50_000);
    }

    #[rstest]
    fn test_data_scale_config_effective_record_count_default() {
        let config = DataScaleConfig::new(DataScale::Large);
        assert_eq!(config.effective_record_count(), 1_000_000);
    }

    #[rstest]
    fn test_data_scale_config_effective_record_count_override() {
        let config = DataScaleConfig {
            scale: DataScale::Large,
            record_count: Some(500_000),
            seed: None,
            incremental: false,
        };
        assert_eq!(config.effective_record_count(), 500_000);
    }

    #[rstest]
    fn test_data_scale_config_with_seed() {
        let config = DataScaleConfig::new(DataScale::Medium).with_seed(42);
        assert_eq!(config.seed, Some(42));
    }

    #[rstest]
    fn test_data_scale_config_with_incremental() {
        let config = DataScaleConfig::new(DataScale::Small).with_incremental(true);
        assert!(config.incremental);
    }

    #[rstest]
    fn test_data_scale_config_default() {
        let config = DataScaleConfig::default();
        assert_eq!(config.scale, DataScale::Medium);
        assert_eq!(config.record_count, None);
        assert_eq!(config.seed, None);
        assert!(!config.incremental);
    }

    #[rstest]
    fn test_data_scale_config_from_data_scale() {
        let config: DataScaleConfig = DataScale::Large.into();
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.effective_record_count(), 1_000_000);
    }

    #[rstest]
    fn test_data_scale_config_serde_minimal() {
        let yaml = r"
scale: large
";
        let config: DataScaleConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, None);
        assert_eq!(config.seed, None);
        assert!(!config.incremental);
    }

    #[rstest]
    fn test_data_scale_config_serde_full() {
        let yaml = r"
scale: large
record_count: 500000
seed: 42
incremental: true
";
        let config: DataScaleConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, Some(500_000));
        assert_eq!(config.seed, Some(42));
        assert!(config.incremental);
        assert_eq!(config.effective_record_count(), 500_000);
    }

    #[rstest]
    fn test_data_scale_config_serde_unknown_field_rejected() {
        let yaml = r"
scale: large
unknown_field: value
";
        let result: Result<DataScaleConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // PayloadVariant Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("minimal", PayloadVariant::Minimal)]
    #[case("min", PayloadVariant::Minimal)]
    #[case("standard", PayloadVariant::Standard)]
    #[case("std", PayloadVariant::Standard)]
    #[case("complex", PayloadVariant::Complex)]
    #[case("heavy", PayloadVariant::Heavy)]
    fn test_payload_variant_from_str_valid(#[case] input: &str, #[case] expected: PayloadVariant) {
        let result: Result<PayloadVariant, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    fn test_payload_variant_counts() {
        assert_eq!(PayloadVariant::Minimal.tag_count(), 0);
        assert_eq!(PayloadVariant::Minimal.subtask_count(), 0);
        assert_eq!(PayloadVariant::Standard.tag_count(), 10);
        assert_eq!(PayloadVariant::Standard.subtask_count(), 10);
        assert_eq!(PayloadVariant::Complex.tag_count(), 100);
        assert_eq!(PayloadVariant::Complex.subtask_count(), 50);
        assert_eq!(PayloadVariant::Heavy.tag_count(), 100);
        assert_eq!(PayloadVariant::Heavy.subtask_count(), 200);
    }

    // -------------------------------------------------------------------------
    // RpsProfile Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("constant", RpsProfile::Constant)]
    #[case("steady", RpsProfile::Constant)]
    #[case("ramp_up_down", RpsProfile::RampUpDown)]
    #[case("burst", RpsProfile::Burst)]
    #[case("step_up", RpsProfile::StepUp)]
    fn test_rps_profile_from_str_valid(#[case] input: &str, #[case] expected: RpsProfile) {
        let result: Result<RpsProfile, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    // -------------------------------------------------------------------------
    // BenchmarkScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_builder() {
        let scenario = BenchmarkScenario::builder("test_scenario")
            .description("A test scenario")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::WriteHeavy)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Complex)
            .rps_profile(RpsProfile::Burst)
            .duration_seconds(120)
            .connections(50)
            .threads(4)
            .build();

        assert_eq!(scenario.name, "test_scenario");
        assert_eq!(scenario.description, "A test scenario");
        assert_eq!(scenario.storage_mode, StorageMode::Postgres);
        assert_eq!(scenario.cache_mode, CacheMode::Redis);
        assert_eq!(scenario.load_pattern, LoadPattern::WriteHeavy);
        assert_eq!(scenario.cache_state, CacheState::Cold);
        assert_eq!(scenario.data_scale, DataScale::Large);
        assert_eq!(scenario.payload_variant, PayloadVariant::Complex);
        assert_eq!(scenario.rps_profile, RpsProfile::Burst);
        assert_eq!(scenario.duration_seconds, 120);
        assert_eq!(scenario.connections, 50);
        assert_eq!(scenario.threads, 4);
    }

    #[rstest]
    fn test_scenario_canonical_name() {
        let scenario = BenchmarkScenario::builder("test")
            .description("test")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        // Full canonical name includes all configuration parameters including contention level
        // Default contention level is "low"
        assert_eq!(
            scenario.canonical_name(),
            "postgres_redis_read_heavy_warm_medium_standard_constant_low"
        );

        // Short canonical name includes only core parameters
        assert_eq!(
            scenario.short_canonical_name(),
            "postgres_redis_read_heavy_warm"
        );
    }

    #[rstest]
    fn test_scenario_canonical_name_with_concurrency() {
        let scenario = BenchmarkScenario::builder("test")
            .description("test")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .contention_level(ContentionLevel::High)
            .concurrency(ConcurrencyConfig::small_pool())
            .build();

        // Canonical name includes contention level and concurrency preset
        assert_eq!(
            scenario.canonical_name(),
            "postgres_redis_read_heavy_warm_medium_standard_constant_high_small"
        );
    }

    #[rstest]
    fn test_scenario_canonical_name_with_custom_concurrency() {
        let custom_concurrency = ConcurrencyConfig {
            worker_threads: 3,
            database_pool_size: 15,
            redis_pool_size: 12,
            max_connections: 75,
        };

        let scenario = BenchmarkScenario::builder("test")
            .description("test")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Burst)
            .contention_level(ContentionLevel::Medium)
            .concurrency(custom_concurrency)
            .build();

        // Custom concurrency should include all configuration values to prevent collision
        assert_eq!(
            scenario.canonical_name(),
            "in_memory_in_memory_mixed_cold_small_minimal_burst_medium_w3d15r12m75"
        );
    }

    #[rstest]
    fn test_scenario_canonical_name_custom_concurrency_no_collision() {
        // Test that different custom concurrency configurations produce different canonical names
        let custom_concurrency_a = ConcurrencyConfig {
            worker_threads: 3,
            database_pool_size: 15,
            redis_pool_size: 12,
            max_connections: 75,
        };

        let custom_concurrency_b = ConcurrencyConfig {
            worker_threads: 4,
            database_pool_size: 20,
            redis_pool_size: 16,
            max_connections: 100,
        };

        let scenario_a = BenchmarkScenario::builder("test_a")
            .description("test")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Burst)
            .contention_level(ContentionLevel::Medium)
            .concurrency(custom_concurrency_a)
            .build();

        let scenario_b = BenchmarkScenario::builder("test_b")
            .description("test")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Burst)
            .contention_level(ContentionLevel::Medium)
            .concurrency(custom_concurrency_b)
            .build();

        // Different custom configurations should produce different canonical names
        assert_ne!(scenario_a.canonical_name(), scenario_b.canonical_name());

        // Verify exact format includes all values
        assert_eq!(
            scenario_a.canonical_name(),
            "in_memory_in_memory_mixed_cold_small_minimal_burst_medium_w3d15r12m75"
        );
        assert_eq!(
            scenario_b.canonical_name(),
            "in_memory_in_memory_mixed_cold_small_minimal_burst_medium_w4d20r16m100"
        );
    }

    #[rstest]
    fn test_scenario_yaml_roundtrip() {
        let scenario = BenchmarkScenario::builder("yaml_test")
            .description("YAML roundtrip test")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Burst)
            .build();

        let yaml = scenario.to_yaml().unwrap();
        let parsed = BenchmarkScenario::from_yaml(&yaml).unwrap();

        assert_eq!(parsed.name, scenario.name);
        assert_eq!(parsed.description, scenario.description);
        assert_eq!(parsed.storage_mode, scenario.storage_mode);
        assert_eq!(parsed.cache_mode, scenario.cache_mode);
        assert_eq!(parsed.load_pattern, scenario.load_pattern);
        assert_eq!(parsed.cache_state, scenario.cache_state);
        assert_eq!(parsed.data_scale, scenario.data_scale);
        assert_eq!(parsed.payload_variant, scenario.payload_variant);
        assert_eq!(parsed.rps_profile, scenario.rps_profile);
    }

    #[rstest]
    fn test_scenario_from_yaml() {
        let yaml = r#"
name: "test_from_yaml"
description: "Test scenario from YAML"
storage_mode: postgres
cache_mode: redis
load_pattern: write_heavy
cache_state: cold
data_scale: large
payload_variant: complex
rps_profile: burst
duration_seconds: 120
connections: 100
threads: 8
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert_eq!(scenario.name, "test_from_yaml");
        assert_eq!(scenario.storage_mode, StorageMode::Postgres);
        assert_eq!(scenario.cache_mode, CacheMode::Redis);
        assert_eq!(scenario.load_pattern, LoadPattern::WriteHeavy);
        assert_eq!(scenario.cache_state, CacheState::Cold);
        assert_eq!(scenario.data_scale, DataScale::Large);
        assert_eq!(scenario.payload_variant, PayloadVariant::Complex);
        assert_eq!(scenario.rps_profile, RpsProfile::Burst);
        assert_eq!(scenario.duration_seconds, 120);
        assert_eq!(scenario.connections, 100);
        assert_eq!(scenario.threads, 8);
    }

    #[rstest]
    fn test_scenario_from_yaml_rejects_unknown_fields() {
        let yaml = r#"
name: "test"
description: "test"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
unknown_field: "should fail"
"#;

        let result = BenchmarkScenario::from_yaml(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown_field"));
    }

    #[rstest]
    fn test_scenario_from_yaml_requires_all_core_fields() {
        // Missing data_scale, payload_variant, rps_profile
        let yaml = r#"
name: "test"
description: "test"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
"#;

        let result = BenchmarkScenario::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_scenario_from_yaml_with_data_scale_config() {
        let yaml = r#"
name: "test_with_data_scale_config"
description: "Test scenario with data_scale_config"
storage_mode: postgres
cache_mode: redis
load_pattern: read_heavy
cache_state: cold
data_scale: large
payload_variant: minimal
rps_profile: step_up
data_scale_config:
  scale: large
  record_count: 500000
  seed: 42
  incremental: false
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert_eq!(scenario.name, "test_with_data_scale_config");
        assert_eq!(scenario.data_scale, DataScale::Large);

        let config = scenario.data_scale_config.unwrap();
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, Some(500_000));
        assert_eq!(config.seed, Some(42));
        assert!(!config.incremental);
        assert_eq!(config.effective_record_count(), 500_000);
    }

    #[rstest]
    fn test_scenario_from_yaml_without_data_scale_config() {
        let yaml = r#"
name: "test_without_data_scale_config"
description: "Test scenario without data_scale_config"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert_eq!(scenario.data_scale, DataScale::Medium);
        assert!(scenario.data_scale_config.is_none());

        let effective_config = scenario.effective_data_scale_config();
        assert_eq!(effective_config.scale, DataScale::Medium);
        assert_eq!(effective_config.effective_record_count(), 10_000);
    }

    #[rstest]
    fn test_scenario_from_yaml_data_scale_config_partial() {
        let yaml = r#"
name: "test_partial_data_scale_config"
description: "Test scenario with partial data_scale_config"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: large
payload_variant: standard
rps_profile: constant
data_scale_config:
  scale: large
  seed: 12345
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        let config = scenario.data_scale_config.unwrap();
        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, None);
        assert_eq!(config.seed, Some(12345));
        assert!(!config.incremental);
        // Without record_count override, uses scale default
        assert_eq!(config.effective_record_count(), 1_000_000);
    }

    // -------------------------------------------------------------------------
    // ScenarioMatrix Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_matrix_generate() {
        let matrix = ScenarioMatrix {
            name_prefix: "bench".to_string(),
            description_template: String::new(),
            storage_modes: vec![StorageMode::InMemory, StorageMode::Postgres],
            cache_modes: vec![CacheMode::InMemory],
            load_patterns: vec![LoadPattern::ReadHeavy, LoadPattern::WriteHeavy],
            cache_states: vec![CacheState::Warm],
            data_scales: vec![DataScale::Medium],
            payload_variants: vec![PayloadVariant::Standard],
            rps_profiles: vec![RpsProfile::Constant],
            default_duration_seconds: 60,
            default_connections: 10,
            default_threads: 2,
        };

        let scenarios = matrix.generate_scenarios();

        // 2 storage modes × 1 cache mode × 2 load patterns × 1 cache state × 1 × 1 × 1 = 4 scenarios
        assert_eq!(scenarios.len(), 4);
        assert_eq!(matrix.combination_count(), 4);

        // Verify first scenario
        assert!(scenarios[0].name.starts_with("bench_"));
        assert_eq!(scenarios[0].storage_mode, StorageMode::InMemory);
        assert_eq!(scenarios[0].data_scale, DataScale::Medium);
        assert_eq!(scenarios[0].payload_variant, PayloadVariant::Standard);
        assert_eq!(scenarios[0].rps_profile, RpsProfile::Constant);
    }

    #[rstest]
    fn test_scenario_matrix_full_matrix() {
        let matrix = ScenarioMatrix {
            name_prefix: "full".to_string(),
            description_template: String::new(),
            storage_modes: vec![StorageMode::InMemory, StorageMode::Postgres],
            cache_modes: vec![CacheMode::InMemory, CacheMode::Redis],
            load_patterns: vec![
                LoadPattern::ReadHeavy,
                LoadPattern::WriteHeavy,
                LoadPattern::Mixed,
            ],
            cache_states: vec![CacheState::Cold, CacheState::Warm],
            data_scales: vec![DataScale::Small, DataScale::Medium],
            payload_variants: vec![PayloadVariant::Standard],
            rps_profiles: vec![RpsProfile::Constant],
            default_duration_seconds: 60,
            default_connections: 10,
            default_threads: 2,
        };

        let scenarios = matrix.generate_scenarios();

        // 2 × 2 × 3 × 2 × 2 × 1 × 1 = 48 scenarios
        assert_eq!(scenarios.len(), 48);
        assert_eq!(matrix.combination_count(), 48);
    }

    #[rstest]
    fn test_scenario_matrix_with_all_dimensions() {
        let matrix = ScenarioMatrix {
            name_prefix: "all".to_string(),
            description_template: "Full matrix test".to_string(),
            storage_modes: vec![StorageMode::InMemory],
            cache_modes: vec![CacheMode::InMemory],
            load_patterns: vec![LoadPattern::ReadHeavy],
            cache_states: vec![CacheState::Warm],
            data_scales: vec![DataScale::Small, DataScale::Medium, DataScale::Large],
            payload_variants: vec![PayloadVariant::Minimal, PayloadVariant::Heavy],
            rps_profiles: vec![RpsProfile::Constant, RpsProfile::Burst],
            default_duration_seconds: 120,
            default_connections: 20,
            default_threads: 4,
        };

        let scenarios = matrix.generate_scenarios();

        // 1 × 1 × 1 × 1 × 3 × 2 × 2 = 12 scenarios
        assert_eq!(scenarios.len(), 12);
        assert_eq!(matrix.combination_count(), 12);

        // Verify description and defaults
        assert_eq!(scenarios[0].description, "Full matrix test");
        assert_eq!(scenarios[0].duration_seconds, 120);
        assert_eq!(scenarios[0].connections, 20);
        assert_eq!(scenarios[0].threads, 4);
    }

    #[rstest]
    fn test_scenario_matrix_from_yaml_rejects_unknown_fields() {
        let yaml = r#"
name_prefix: "test"
storage_modes:
  - in_memory
cache_modes:
  - in_memory
load_patterns:
  - read_heavy
cache_states:
  - warm
unknown_field: "should fail"
"#;

        let result: Result<ScenarioMatrix, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown_field"));
    }

    #[rstest]
    fn test_scenario_matrix_from_yaml_catches_typo_in_optional_field() {
        // Typo: "data_scale" instead of "data_scales" (missing 's')
        // Without deny_unknown_fields, this would silently fall back to default
        let yaml = r#"
name_prefix: "test"
storage_modes:
  - in_memory
cache_modes:
  - in_memory
load_patterns:
  - read_heavy
cache_states:
  - warm
data_scale:
  - large
"#;

        let result: Result<ScenarioMatrix, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("data_scale"));
    }

    // -------------------------------------------------------------------------
    // PoolConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.database_pool_size, 10);
        assert_eq!(config.redis_pool_size, 10);
    }

    // -------------------------------------------------------------------------
    // ConcurrencyConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_concurrency_config_default() {
        let config = ConcurrencyConfig::default();
        assert_eq!(config.worker_threads, 4);
        assert_eq!(config.database_pool_size, 10);
        assert_eq!(config.redis_pool_size, 10);
        assert_eq!(config.max_connections, 100);
    }

    #[rstest]
    fn test_concurrency_config_small_pool() {
        let config = ConcurrencyConfig::small_pool();
        assert_eq!(config.worker_threads, 1);
        assert_eq!(config.database_pool_size, 4);
        assert_eq!(config.redis_pool_size, 4);
        assert_eq!(config.max_connections, 50);
    }

    #[rstest]
    fn test_concurrency_config_medium_pool() {
        let config = ConcurrencyConfig::medium_pool();
        assert_eq!(config.worker_threads, 4);
        assert_eq!(config.database_pool_size, 8);
        assert_eq!(config.redis_pool_size, 8);
        assert_eq!(config.max_connections, 100);
    }

    #[rstest]
    fn test_concurrency_config_large_pool() {
        let config = ConcurrencyConfig::large_pool();
        assert_eq!(config.worker_threads, 8);
        assert_eq!(config.database_pool_size, 32);
        assert_eq!(config.redis_pool_size, 32);
        assert_eq!(config.max_connections, 200);
    }

    #[rstest]
    fn test_concurrency_config_description() {
        let config = ConcurrencyConfig::small_pool();
        let description = config.description();
        assert!(description.contains("workers=1"));
        assert!(description.contains("db_pool=4"));
        assert!(description.contains("redis_pool=4"));
        assert!(description.contains("max_conn=50"));
    }

    #[rstest]
    fn test_concurrency_config_yaml_roundtrip() {
        let config = ConcurrencyConfig {
            worker_threads: 8,
            database_pool_size: 32,
            redis_pool_size: 16,
            max_connections: 200,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: ConcurrencyConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(parsed.worker_threads, 8);
        assert_eq!(parsed.database_pool_size, 32);
        assert_eq!(parsed.redis_pool_size, 16);
        assert_eq!(parsed.max_connections, 200);
    }

    #[rstest]
    fn test_concurrency_config_yaml_partial_defaults() {
        let yaml = r"
worker_threads: 2
";

        let parsed: ConcurrencyConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(parsed.worker_threads, 2);
        assert_eq!(parsed.database_pool_size, 10);
        assert_eq!(parsed.redis_pool_size, 10);
        assert_eq!(parsed.max_connections, 100);
    }

    // -------------------------------------------------------------------------
    // ContentionLevel Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("low", ContentionLevel::Low)]
    #[case("read_heavy", ContentionLevel::Low)]
    #[case("distributed", ContentionLevel::Low)]
    #[case("medium", ContentionLevel::Medium)]
    #[case("balanced", ContentionLevel::Medium)]
    #[case("mixed", ContentionLevel::Medium)]
    #[case("high", ContentionLevel::High)]
    #[case("write_heavy", ContentionLevel::High)]
    #[case("concentrated", ContentionLevel::High)]
    fn test_contention_level_from_str_valid(
        #[case] input: &str,
        #[case] expected: ContentionLevel,
    ) {
        let result: Result<ContentionLevel, _> = input.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[rstest]
    #[case("invalid")]
    #[case("")]
    #[case("extreme")]
    fn test_contention_level_from_str_invalid(#[case] input: &str) {
        let result: Result<ContentionLevel, _> = input.parse();
        assert!(result.is_err());
    }

    #[rstest]
    fn test_contention_level_ratios() {
        assert_eq!(ContentionLevel::Low.read_ratio(), 90);
        assert_eq!(ContentionLevel::Low.write_ratio(), 10);
        assert_eq!(ContentionLevel::Medium.read_ratio(), 50);
        assert_eq!(ContentionLevel::Medium.write_ratio(), 50);
        assert_eq!(ContentionLevel::High.read_ratio(), 10);
        assert_eq!(ContentionLevel::High.write_ratio(), 90);
    }

    #[rstest]
    fn test_contention_level_target_resource_count() {
        assert_eq!(ContentionLevel::Low.target_resource_count(), 1000);
        assert_eq!(ContentionLevel::Medium.target_resource_count(), 100);
        assert_eq!(ContentionLevel::High.target_resource_count(), 10);
    }

    #[rstest]
    fn test_contention_level_description() {
        assert!(ContentionLevel::Low.description().contains("read-heavy"));
        assert!(ContentionLevel::Medium.description().contains("balanced"));
        assert!(ContentionLevel::High.description().contains("write-heavy"));
    }

    #[rstest]
    fn test_contention_level_default() {
        assert_eq!(ContentionLevel::default(), ContentionLevel::Low);
    }

    #[rstest]
    fn test_contention_level_display() {
        assert_eq!(ContentionLevel::Low.to_string(), "low");
        assert_eq!(ContentionLevel::Medium.to_string(), "medium");
        assert_eq!(ContentionLevel::High.to_string(), "high");
    }

    #[rstest]
    fn test_contention_level_yaml_roundtrip() {
        for level in [
            ContentionLevel::Low,
            ContentionLevel::Medium,
            ContentionLevel::High,
        ] {
            let yaml = serde_yaml::to_string(&level).unwrap();
            let parsed: ContentionLevel = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(parsed, level);
        }
    }

    // -------------------------------------------------------------------------
    // Scenario with Concurrency/Contention Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_with_concurrency_config() {
        let scenario = BenchmarkScenario::builder("concurrency_test")
            .description("Test with concurrency config")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::WriteHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .concurrency(ConcurrencyConfig::small_pool())
            .contention_level(ContentionLevel::High)
            .build();

        assert!(scenario.concurrency.is_some());
        let concurrency = scenario.concurrency.unwrap();
        assert_eq!(concurrency.worker_threads, 1);
        assert_eq!(concurrency.database_pool_size, 4);

        assert_eq!(scenario.contention_level, ContentionLevel::High);
    }

    #[rstest]
    fn test_scenario_with_concurrency_yaml_parsing() {
        let yaml = r#"
name: "concurrency_yaml_test"
description: "Test parsing concurrency from YAML"
storage_mode: postgres
cache_mode: redis
load_pattern: write_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
concurrency:
  worker_threads: 2
  database_pool_size: 8
  redis_pool_size: 4
  max_connections: 50
contention_level: high
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.concurrency.is_some());
        let concurrency = scenario.concurrency.unwrap();
        assert_eq!(concurrency.worker_threads, 2);
        assert_eq!(concurrency.database_pool_size, 8);
        assert_eq!(concurrency.redis_pool_size, 4);
        assert_eq!(concurrency.max_connections, 50);

        assert_eq!(scenario.contention_level, ContentionLevel::High);
    }

    #[rstest]
    fn test_scenario_concurrency_yaml_roundtrip() {
        let scenario = BenchmarkScenario::builder("roundtrip_test")
            .description("Roundtrip test")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Heavy)
            .rps_profile(RpsProfile::Burst)
            .concurrency(ConcurrencyConfig::large_pool())
            .contention_level(ContentionLevel::Medium)
            .build();

        let yaml = scenario.to_yaml().unwrap();
        let parsed = BenchmarkScenario::from_yaml(&yaml).unwrap();

        assert!(parsed.concurrency.is_some());
        let concurrency = parsed.concurrency.unwrap();
        assert_eq!(concurrency.worker_threads, 8);
        assert_eq!(concurrency.database_pool_size, 32);

        assert_eq!(parsed.contention_level, ContentionLevel::Medium);
    }

    // -------------------------------------------------------------------------
    // WorkerConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_worker_config_default() {
        let config = WorkerConfig::default();
        assert_eq!(config.worker_threads, 4);
    }

    // -------------------------------------------------------------------------
    // Thresholds Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_thresholds_default() {
        let thresholds = Thresholds::default();
        assert!((thresholds.max_error_rate - 0.01).abs() < f64::EPSILON);
        assert_eq!(thresholds.p99_latency_ms, 100);
        assert_eq!(thresholds.min_rps_achieved, 0);
    }

    #[rstest]
    fn test_thresholds_yaml_roundtrip() {
        let thresholds = Thresholds {
            max_error_rate: 0.05,
            p99_latency_ms: 200,
            min_rps_achieved: 1000,
        };

        let yaml = serde_yaml::to_string(&thresholds).unwrap();
        let parsed: Thresholds = serde_yaml::from_str(&yaml).unwrap();

        assert!((parsed.max_error_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(parsed.p99_latency_ms, 200);
        assert_eq!(parsed.min_rps_achieved, 1000);
    }

    #[rstest]
    fn test_thresholds_yaml_partial_defaults() {
        // When only some fields are specified, others should use Default values
        let yaml = r"
max_error_rate: 0.02
";

        let parsed: Thresholds = serde_yaml::from_str(yaml).unwrap();

        assert!((parsed.max_error_rate - 0.02).abs() < f64::EPSILON);
        // These should use Default values, not 0
        assert_eq!(parsed.p99_latency_ms, 100);
        assert_eq!(parsed.min_rps_achieved, 0);
    }

    #[rstest]
    fn test_thresholds_yaml_empty_uses_defaults() {
        // Empty YAML should use all Default values
        let yaml = "{}";

        let parsed: Thresholds = serde_yaml::from_str(yaml).unwrap();

        assert!((parsed.max_error_rate - 0.01).abs() < f64::EPSILON);
        assert_eq!(parsed.p99_latency_ms, 100);
        assert_eq!(parsed.min_rps_achieved, 0);
    }

    // -------------------------------------------------------------------------
    // ScenarioMetadata Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_metadata_default() {
        let metadata = ScenarioMetadata::default();
        assert!(metadata.test_type.is_empty());
        assert!(metadata.payload_variant.is_empty());
        assert_eq!(metadata.payload_size_bytes, 0);
        assert_eq!(metadata.tag_count, 0);
        assert_eq!(metadata.subtask_count, 0);
        assert!(metadata.purpose.is_empty());
        assert!(metadata.tags.is_empty());
    }

    #[rstest]
    fn test_scenario_metadata_yaml_roundtrip() {
        let metadata = ScenarioMetadata {
            test_type: "stress_test".to_string(),
            payload_variant: "heavy".to_string(),
            payload_size_bytes: 10240,
            tag_count: 100,
            subtask_count: 50,
            purpose: "Test high load".to_string(),
            tags: vec!["performance".to_string(), "stress".to_string()],
        };

        let yaml = serde_yaml::to_string(&metadata).unwrap();
        let parsed: ScenarioMetadata = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(parsed.test_type, "stress_test");
        assert_eq!(parsed.payload_variant, "heavy");
        assert_eq!(parsed.payload_size_bytes, 10240);
        assert_eq!(parsed.tag_count, 100);
        assert_eq!(parsed.subtask_count, 50);
        assert_eq!(parsed.purpose, "Test high load");
        assert_eq!(parsed.tags, vec!["performance", "stress"]);
    }

    #[rstest]
    fn test_scenario_metadata_yaml_partial_defaults() {
        let yaml = r#"
test_type: "payload_variation"
purpose: "Test different payloads"
"#;

        let parsed: ScenarioMetadata = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(parsed.test_type, "payload_variation");
        assert_eq!(parsed.purpose, "Test different payloads");
        // These should use Default values
        assert!(parsed.payload_variant.is_empty());
        assert_eq!(parsed.payload_size_bytes, 0);
        assert_eq!(parsed.tag_count, 0);
        assert_eq!(parsed.subtask_count, 0);
        assert!(parsed.tags.is_empty());
    }

    // -------------------------------------------------------------------------
    // http_methods Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_http_methods_yaml_roundtrip() {
        let scenario = BenchmarkScenario::builder("http_methods_test")
            .description("Test HTTP methods")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .http_methods(vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
            ])
            .build();

        let yaml = scenario.to_yaml().unwrap();
        let parsed = BenchmarkScenario::from_yaml(&yaml).unwrap();

        assert_eq!(parsed.http_methods, vec!["GET", "POST", "PUT"]);
    }

    #[rstest]
    fn test_http_methods_yaml_parsing() {
        let yaml = r#"
name: "http_methods_parse_test"
description: "Test parsing HTTP methods from YAML"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
http_methods:
  - GET
  - POST
  - DELETE
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert_eq!(scenario.http_methods, vec!["GET", "POST", "DELETE"]);
    }

    #[rstest]
    fn test_http_methods_default_empty() {
        let yaml = r#"
name: "no_http_methods"
description: "Scenario without HTTP methods"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.http_methods.is_empty());
    }

    // -------------------------------------------------------------------------
    // Thresholds in BenchmarkScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_thresholds_yaml_parsing() {
        let yaml = r#"
name: "thresholds_test"
description: "Test thresholds parsing"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
thresholds:
  max_error_rate: 0.05
  p99_latency_ms: 150
  min_rps_achieved: 500
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.thresholds.is_some());
        let thresholds = scenario.thresholds.unwrap();
        assert!((thresholds.max_error_rate - 0.05).abs() < f64::EPSILON);
        assert_eq!(thresholds.p99_latency_ms, 150);
        assert_eq!(thresholds.min_rps_achieved, 500);
    }

    #[rstest]
    fn test_scenario_thresholds_partial_defaults() {
        let yaml = r#"
name: "thresholds_partial_test"
description: "Test thresholds with partial defaults"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
thresholds:
  max_error_rate: 0.02
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.thresholds.is_some());
        let thresholds = scenario.thresholds.unwrap();
        assert!((thresholds.max_error_rate - 0.02).abs() < f64::EPSILON);
        // These should use Default values from Thresholds::default()
        assert_eq!(thresholds.p99_latency_ms, 100);
        assert_eq!(thresholds.min_rps_achieved, 0);
    }

    // -------------------------------------------------------------------------
    // Metadata in BenchmarkScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_metadata_yaml_parsing() {
        let yaml = r#"
name: "metadata_test"
description: "Test metadata parsing"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
metadata:
  test_type: "stress_test"
  payload_variant: "heavy"
  payload_size_bytes: 5000
  tag_count: 50
  subtask_count: 25
  purpose: "Load testing"
  tags:
    - performance
    - load
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.metadata.is_some());
        let metadata = scenario.metadata.unwrap();
        assert_eq!(metadata.test_type, "stress_test");
        assert_eq!(metadata.payload_variant, "heavy");
        assert_eq!(metadata.payload_size_bytes, 5000);
        assert_eq!(metadata.tag_count, 50);
        assert_eq!(metadata.subtask_count, 25);
        assert_eq!(metadata.purpose, "Load testing");
        assert_eq!(metadata.tags, vec!["performance", "load"]);
    }

    // -------------------------------------------------------------------------
    // Builder Tests for New Fields
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_builder_http_methods() {
        let scenario = BenchmarkScenario::builder("builder_http_methods_test")
            .description("Test builder with HTTP methods")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .http_methods(vec!["POST".to_string(), "PATCH".to_string()])
            .build();

        assert_eq!(scenario.http_methods, vec!["POST", "PATCH"]);
    }

    // -------------------------------------------------------------------------
    // Error Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_error_display() {
        let error = ScenarioError::InvalidLoadPattern("bad".to_string());
        assert!(error.to_string().contains("bad"));
        assert!(error.to_string().contains("load pattern"));

        let error = ScenarioError::InvalidCacheState("bad".to_string());
        assert!(error.to_string().contains("cache state"));

        let error = ScenarioError::InvalidContentionLevel("bad".to_string());
        assert!(error.to_string().contains("bad"));
        assert!(error.to_string().contains("contention level"));

        let error = ScenarioError::FileRead("not found".to_string());
        assert!(error.to_string().contains("read"));

        let error = ScenarioError::YamlParse("syntax error".to_string());
        assert!(error.to_string().contains("parse"));
    }

    // -------------------------------------------------------------------------
    // YAML File Smoke Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("in_memory_read_heavy_warm")]
    #[case("postgres_redis_read_heavy_warm")]
    #[case("postgres_redis_read_heavy_cold")]
    #[case("postgres_write_heavy")]
    #[case("mixed_workload_burst")]
    #[case("large_scale_read")]
    #[case("large_scale_seeded")]
    #[case("pool_stress_small")]
    #[case("pool_stress_medium")]
    #[case("pool_stress_large")]
    #[case("write_contention_high")]
    #[case("mixed_contention")]
    #[case("profiling_baseline")]
    #[case("cache_warm_test")]
    #[case("cache_cold_test")]
    fn test_scenario_yaml_files_parse_successfully(#[case] scenario_name: &str) {
        // This test verifies that all YAML scenario files in the benchmarks/scenarios
        // directory can be parsed without errors.
        // CARGO_MANIFEST_DIR points to benches/api, so path is relative from there.
        let yaml_path = format!(
            "{}/benchmarks/scenarios/{}.yaml",
            env!("CARGO_MANIFEST_DIR"),
            scenario_name
        );

        let result = BenchmarkScenario::from_file(&yaml_path);

        assert!(
            result.is_ok(),
            "Failed to parse {}.yaml: {:?}",
            scenario_name,
            result.err()
        );

        let scenario = result.unwrap();
        assert_eq!(scenario.name, scenario_name);
        assert!(!scenario.description.is_empty());
    }

    // -------------------------------------------------------------------------
    // Concurrency Scenario YAML Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("pool_stress_small", 1, 4, ContentionLevel::Medium)]
    #[case("pool_stress_medium", 4, 8, ContentionLevel::Medium)]
    #[case("pool_stress_large", 8, 32, ContentionLevel::Low)]
    #[case("write_contention_high", 4, 8, ContentionLevel::High)]
    #[case("mixed_contention", 4, 16, ContentionLevel::Medium)]
    fn test_concurrency_scenario_yaml_values(
        #[case] scenario_name: &str,
        #[case] expected_workers: u32,
        #[case] expected_db_pool: u32,
        #[case] expected_contention: ContentionLevel,
    ) {
        let yaml_path = format!(
            "{}/benchmarks/scenarios/{}.yaml",
            env!("CARGO_MANIFEST_DIR"),
            scenario_name
        );

        let scenario = BenchmarkScenario::from_file(&yaml_path).unwrap();

        // Verify concurrency configuration
        assert!(
            scenario.concurrency.is_some(),
            "{scenario_name} should have concurrency config"
        );
        let concurrency = scenario.concurrency.unwrap();
        assert_eq!(
            concurrency.worker_threads, expected_workers,
            "{scenario_name} worker_threads mismatch"
        );
        assert_eq!(
            concurrency.database_pool_size, expected_db_pool,
            "{scenario_name} database_pool_size mismatch"
        );

        // Verify contention level (now a required field with default)
        assert_eq!(
            scenario.contention_level, expected_contention,
            "{scenario_name} contention_level mismatch"
        );
    }

    // -------------------------------------------------------------------------
    // ConcurrencyConfig preset_name Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_concurrency_config_preset_name_small() {
        let config = ConcurrencyConfig::small_pool();
        assert_eq!(config.preset_name(), "small");
    }

    #[rstest]
    fn test_concurrency_config_preset_name_medium() {
        let config = ConcurrencyConfig::medium_pool();
        assert_eq!(config.preset_name(), "medium");
    }

    #[rstest]
    fn test_concurrency_config_preset_name_large() {
        let config = ConcurrencyConfig::large_pool();
        assert_eq!(config.preset_name(), "large");
    }

    #[rstest]
    fn test_concurrency_config_preset_name_custom() {
        let config = ConcurrencyConfig {
            worker_threads: 3,
            database_pool_size: 15,
            redis_pool_size: 12,
            max_connections: 75,
        };
        assert_eq!(config.preset_name(), "custom");
    }

    // -------------------------------------------------------------------------
    // to_env_vars Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_to_env_vars_basic() {
        let scenario = BenchmarkScenario::builder("env_test")
            .description("Test environment variables")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::WriteHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .contention_level(ContentionLevel::High)
            .build();

        let env_vars = scenario.to_env_vars();

        // Check contention level settings
        assert!(env_vars.contains(&("CONTENTION_LEVEL".to_string(), "high".to_string())));
        assert!(env_vars.contains(&("WRITE_RATIO".to_string(), "90".to_string())));
        assert!(env_vars.contains(&("TARGET_RESOURCES".to_string(), "10".to_string())));

        // Check load generation parameters
        assert!(env_vars.contains(&("CONNECTIONS".to_string(), "10".to_string())));
        assert!(env_vars.contains(&("THREADS".to_string(), "2".to_string())));
        assert!(env_vars.contains(&("DURATION_SECONDS".to_string(), "60".to_string())));

        // No concurrency config, so no concurrency-related env vars
        assert!(!env_vars.iter().any(|(k, _)| k == "WORKER_THREADS"));
    }

    #[rstest]
    fn test_to_env_vars_with_concurrency() {
        let scenario = BenchmarkScenario::builder("env_concurrency_test")
            .description("Test environment variables with concurrency")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::WriteHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .concurrency(ConcurrencyConfig::small_pool())
            .contention_level(ContentionLevel::Medium)
            .build();

        let env_vars = scenario.to_env_vars();

        // Check concurrency settings
        assert!(env_vars.contains(&("WORKER_THREADS".to_string(), "1".to_string())));
        assert!(env_vars.contains(&("DATABASE_POOL_SIZE".to_string(), "4".to_string())));
        assert!(env_vars.contains(&("REDIS_POOL_SIZE".to_string(), "4".to_string())));
        assert!(env_vars.contains(&("MAX_CONNECTIONS".to_string(), "50".to_string())));

        // Check contention level settings for medium
        assert!(env_vars.contains(&("CONTENTION_LEVEL".to_string(), "medium".to_string())));
        assert!(env_vars.contains(&("WRITE_RATIO".to_string(), "50".to_string())));
        assert!(env_vars.contains(&("TARGET_RESOURCES".to_string(), "100".to_string())));
    }

    #[rstest]
    fn test_to_env_vars_with_target_rps() {
        let scenario = BenchmarkScenario::builder("env_rps_test")
            .description("Test environment variables with target RPS")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Constant)
            .target_rps(1000)
            .build();

        let env_vars = scenario.to_env_vars();

        // Check target RPS is included when > 0
        assert!(env_vars.contains(&("TARGET_RPS".to_string(), "1000".to_string())));
    }

    #[rstest]
    fn test_to_env_vars_no_target_rps_when_zero() {
        let scenario = BenchmarkScenario::builder("env_no_rps_test")
            .description("Test no target RPS in env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Constant)
            .build();

        let env_vars = scenario.to_env_vars();

        // Target RPS should not be included when 0
        assert!(!env_vars.iter().any(|(k, _)| k == "TARGET_RPS"));
    }

    #[rstest]
    fn test_to_env_vars_all_contention_levels() {
        for (level, expected_write_ratio, expected_resources) in [
            (ContentionLevel::Low, "10", "1000"),
            (ContentionLevel::Medium, "50", "100"),
            (ContentionLevel::High, "90", "10"),
        ] {
            let scenario = BenchmarkScenario::builder("contention_test")
                .description("Test contention levels")
                .storage_mode(StorageMode::InMemory)
                .cache_mode(CacheMode::InMemory)
                .load_pattern(LoadPattern::ReadHeavy)
                .cache_state(CacheState::Warm)
                .data_scale(DataScale::Small)
                .payload_variant(PayloadVariant::Minimal)
                .rps_profile(RpsProfile::Constant)
                .contention_level(level)
                .build();

            let env_vars = scenario.to_env_vars();

            assert!(
                env_vars.contains(&("WRITE_RATIO".to_string(), expected_write_ratio.to_string())),
                "Expected WRITE_RATIO={expected_write_ratio} for {level:?}"
            );
            assert!(
                env_vars.contains(&(
                    "TARGET_RESOURCES".to_string(),
                    expected_resources.to_string()
                )),
                "Expected TARGET_RESOURCES={expected_resources} for {level:?}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // contention_level default value test
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_contention_level_defaults_to_low() {
        let yaml = r#"
name: "default_contention_test"
description: "Test default contention level"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        // contention_level should default to Low when not specified
        assert_eq!(scenario.contention_level, ContentionLevel::Low);
    }

    // -------------------------------------------------------------------------
    // ProfilingConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_profiling_config_default() {
        let config = ProfilingConfig::default();

        assert!(!config.enable_perf);
        assert!(!config.enable_flamegraph);
        assert_eq!(config.frequency, 99);
        assert_eq!(config.output_dir, "profiling-results");
    }

    #[rstest]
    fn test_profiling_config_with_perf() {
        let config = ProfilingConfig::with_perf();

        assert!(config.enable_perf);
        assert!(!config.enable_flamegraph);
        assert_eq!(config.frequency, 99);
    }

    #[rstest]
    fn test_profiling_config_with_flamegraph() {
        let config = ProfilingConfig::with_flamegraph();

        assert!(config.enable_perf);
        assert!(config.enable_flamegraph);
        assert_eq!(config.frequency, 99);
    }

    #[rstest]
    fn test_profiling_config_is_enabled() {
        let default_config = ProfilingConfig::default();
        assert!(!default_config.is_enabled());

        let perf_config = ProfilingConfig::with_perf();
        assert!(perf_config.is_enabled());

        let flamegraph_config = ProfilingConfig::with_flamegraph();
        assert!(flamegraph_config.is_enabled());
    }

    #[rstest]
    fn test_profiling_config_serde() {
        let yaml = r#"
enable_perf: true
enable_flamegraph: true
frequency: 199
output_dir: "custom-profiling"
"#;

        let config: ProfilingConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enable_perf);
        assert!(config.enable_flamegraph);
        assert_eq!(config.frequency, 199);
        assert_eq!(config.output_dir, "custom-profiling");
    }

    #[rstest]
    fn test_profiling_config_serde_defaults() {
        let yaml = r"
enable_perf: true
";

        let config: ProfilingConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enable_perf);
        assert!(!config.enable_flamegraph);
        assert_eq!(config.frequency, 99);
        assert_eq!(config.output_dir, "profiling-results");
    }

    // -------------------------------------------------------------------------
    // BenchmarkScenario with ProfilingConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_benchmark_scenario_with_profiling() {
        let yaml = r#"
name: "profiling_test"
description: "Test scenario with profiling"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: mixed
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
profiling:
  enable_perf: true
  enable_flamegraph: true
  frequency: 99
  output_dir: "profiling-results"
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.profiling.enable_perf);
        assert!(scenario.profiling.enable_flamegraph);
        assert_eq!(scenario.profiling.frequency, 99);
        assert_eq!(scenario.profiling.output_dir, "profiling-results");
    }

    #[rstest]
    fn test_benchmark_scenario_profiling_defaults() {
        let yaml = r#"
name: "no_profiling_test"
description: "Test scenario without profiling config"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: mixed
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        // profiling should default to disabled
        assert!(!scenario.profiling.enable_perf);
        assert!(!scenario.profiling.enable_flamegraph);
        assert_eq!(scenario.profiling.frequency, 99);
        assert_eq!(scenario.profiling.output_dir, "profiling-results");
    }

    #[rstest]
    fn test_to_env_vars_includes_profiling() {
        let scenario = BenchmarkScenario::builder("profiling_env_test")
            .description("Test profiling env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .profiling(ProfilingConfig::with_flamegraph())
            .build();

        let env_vars = scenario.to_env_vars();

        assert!(
            env_vars.contains(&("ENABLE_PERF".to_string(), "1".to_string())),
            "Expected ENABLE_PERF=1"
        );
        assert!(
            env_vars.contains(&("ENABLE_FLAMEGRAPH".to_string(), "1".to_string())),
            "Expected ENABLE_FLAMEGRAPH=1"
        );
        assert!(
            env_vars.contains(&("PERF_FREQUENCY".to_string(), "99".to_string())),
            "Expected PERF_FREQUENCY=99"
        );
        assert!(
            env_vars.contains(&(
                "PROFILING_OUTPUT_DIR".to_string(),
                "profiling-results".to_string()
            )),
            "Expected PROFILING_OUTPUT_DIR=profiling-results"
        );
    }

    #[rstest]
    fn test_to_env_vars_profiling_disabled() {
        let scenario = BenchmarkScenario::builder("no_profiling_env_test")
            .description("Test no profiling env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let env_vars = scenario.to_env_vars();

        // ENABLE_PERF and ENABLE_FLAMEGRAPH should not be present when disabled
        assert!(
            !env_vars.contains(&("ENABLE_PERF".to_string(), "1".to_string())),
            "ENABLE_PERF should not be set when disabled"
        );
        assert!(
            !env_vars.contains(&("ENABLE_FLAMEGRAPH".to_string(), "1".to_string())),
            "ENABLE_FLAMEGRAPH should not be set when disabled"
        );

        // But frequency and output_dir should still be present
        assert!(
            env_vars.contains(&("PERF_FREQUENCY".to_string(), "99".to_string())),
            "PERF_FREQUENCY should be present"
        );
        assert!(
            env_vars.contains(&(
                "PROFILING_OUTPUT_DIR".to_string(),
                "profiling-results".to_string()
            )),
            "PROFILING_OUTPUT_DIR should be present"
        );
    }

    #[rstest]
    fn test_to_env_vars_includes_data_scale_default() {
        let scenario = BenchmarkScenario::builder("data_scale_env_test")
            .description("Test data scale env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let env_vars = scenario.to_env_vars();

        assert!(
            env_vars.contains(&("DATA_SCALE".to_string(), "large".to_string())),
            "Expected DATA_SCALE=large"
        );
        assert!(
            env_vars.contains(&("RECORD_COUNT".to_string(), "1000000".to_string())),
            "Expected RECORD_COUNT=1000000 (default for large)"
        );
        // RANDOM_SEED should not be present when not specified
        assert!(
            !env_vars.iter().any(|(key, _)| key == "RANDOM_SEED"),
            "RANDOM_SEED should not be present when not specified"
        );
        // INCREMENTAL should not be present when false
        assert!(
            !env_vars.iter().any(|(key, _)| key == "INCREMENTAL"),
            "INCREMENTAL should not be present when false"
        );
    }

    #[rstest]
    fn test_to_env_vars_includes_data_scale_config() {
        let data_scale_config = DataScaleConfig {
            scale: DataScale::Large,
            record_count: Some(500_000),
            seed: Some(42),
            incremental: true,
        };

        let scenario = BenchmarkScenario::builder("data_scale_config_env_test")
            .description("Test data scale config env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium) // This should be overridden by data_scale_config
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .data_scale_config(data_scale_config)
            .build();

        let env_vars = scenario.to_env_vars();

        assert!(
            env_vars.contains(&("DATA_SCALE".to_string(), "large".to_string())),
            "Expected DATA_SCALE=large (from data_scale_config)"
        );
        assert!(
            env_vars.contains(&("RECORD_COUNT".to_string(), "500000".to_string())),
            "Expected RECORD_COUNT=500000 (custom override)"
        );
        assert!(
            env_vars.contains(&("RANDOM_SEED".to_string(), "42".to_string())),
            "Expected RANDOM_SEED=42"
        );
        assert!(
            env_vars.contains(&("INCREMENTAL".to_string(), "1".to_string())),
            "Expected INCREMENTAL=1"
        );
    }

    #[rstest]
    fn test_effective_data_scale_config_without_config() {
        let scenario = BenchmarkScenario::builder("effective_config_test")
            .description("Test effective data scale config")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let config = scenario.effective_data_scale_config();

        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, None);
        assert_eq!(config.seed, None);
        assert!(!config.incremental);
        assert_eq!(config.effective_record_count(), 1_000_000);
    }

    #[rstest]
    fn test_effective_data_scale_config_with_config() {
        let data_scale_config = DataScaleConfig {
            scale: DataScale::Large,
            record_count: Some(500_000),
            seed: Some(42),
            incremental: true,
        };

        let scenario = BenchmarkScenario::builder("effective_config_override_test")
            .description("Test effective data scale config with override")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .data_scale_config(data_scale_config)
            .build();

        let config = scenario.effective_data_scale_config();

        assert_eq!(config.scale, DataScale::Large);
        assert_eq!(config.record_count, Some(500_000));
        assert_eq!(config.seed, Some(42));
        assert!(config.incremental);
        assert_eq!(config.effective_record_count(), 500_000);
    }

    #[rstest]
    fn test_builder_with_profiling() {
        let scenario = BenchmarkScenario::builder("builder_profiling_test")
            .description("Test builder with profiling")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .profiling(ProfilingConfig {
                enable_perf: true,
                enable_flamegraph: false,
                frequency: 199,
                output_dir: "custom-output".to_string(),
            })
            .build();

        assert!(scenario.profiling.enable_perf);
        assert!(!scenario.profiling.enable_flamegraph);
        assert_eq!(scenario.profiling.frequency, 199);
        assert_eq!(scenario.profiling.output_dir, "custom-output");
    }

    // -------------------------------------------------------------------------
    // CacheMetricsConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_cache_metrics_config_default() {
        let config = CacheMetricsConfig::default();

        // Default is disabled to avoid overhead in existing scenarios
        assert!(!config.enabled);
        assert!(!config.per_endpoint);
        assert!(!config.track_latency);
        assert_eq!(config.warmup_requests, 0);
        assert_eq!(config.expected_hit_rate, None);
    }

    #[rstest]
    fn test_cache_metrics_config_cold_cache() {
        let config = CacheMetricsConfig::cold_cache();

        assert!(config.enabled);
        assert!(config.per_endpoint);
        assert!(config.track_latency);
        assert_eq!(config.warmup_requests, 0);
        assert_eq!(config.expected_hit_rate, Some(0.0));
    }

    #[rstest]
    fn test_cache_metrics_config_warm_cache() {
        let config = CacheMetricsConfig::warm_cache(1000);

        assert!(config.enabled);
        assert!(config.per_endpoint);
        assert!(config.track_latency);
        assert_eq!(config.warmup_requests, 1000);
        assert_eq!(config.expected_hit_rate, Some(0.8));
    }

    #[rstest]
    fn test_cache_metrics_config_is_enabled() {
        // Default is disabled
        let default_config = CacheMetricsConfig::default();
        assert!(!default_config.is_enabled());

        // Explicitly enabled
        let enabled_config = CacheMetricsConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(enabled_config.is_enabled());

        // Using factory methods that enable by default
        let warm_cache = CacheMetricsConfig::warm_cache(100);
        assert!(warm_cache.is_enabled());
    }

    #[rstest]
    fn test_cache_metrics_config_serde_minimal() {
        let yaml = r"
enabled: true
";
        let config: CacheMetricsConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert!(!config.per_endpoint);
        assert!(!config.track_latency);
        assert_eq!(config.warmup_requests, 0);
        assert_eq!(config.expected_hit_rate, None);
    }

    #[rstest]
    fn test_cache_metrics_config_serde_full() {
        let yaml = r"
enabled: true
per_endpoint: true
track_latency: true
warmup_requests: 500
expected_hit_rate: 0.75
";
        let config: CacheMetricsConfig = serde_yaml::from_str(yaml).unwrap();

        assert!(config.enabled);
        assert!(config.per_endpoint);
        assert!(config.track_latency);
        assert_eq!(config.warmup_requests, 500);
        assert_eq!(config.expected_hit_rate, Some(0.75));
    }

    #[rstest]
    fn test_cache_metrics_config_serde_unknown_field_rejected() {
        let yaml = r"
enabled: true
unknown_field: value
";
        let result: Result<CacheMetricsConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_benchmark_scenario_with_cache_metrics() {
        let yaml = r#"
name: "cache_metrics_test"
description: "Test scenario with cache metrics"
storage_mode: postgres
cache_mode: redis
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
cache_metrics:
  enabled: true
  per_endpoint: true
  track_latency: true
  warmup_requests: 1000
  expected_hit_rate: 0.8
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.cache_metrics.enabled);
        assert!(scenario.cache_metrics.per_endpoint);
        assert!(scenario.cache_metrics.track_latency);
        assert_eq!(scenario.cache_metrics.warmup_requests, 1000);
        assert_eq!(scenario.cache_metrics.expected_hit_rate, Some(0.8));
    }

    #[rstest]
    fn test_benchmark_scenario_cache_metrics_defaults() {
        let yaml = r#"
name: "no_cache_metrics_test"
description: "Test scenario without cache metrics config"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        // cache_metrics should default to disabled with other defaults
        assert!(!scenario.cache_metrics.enabled);
        assert!(!scenario.cache_metrics.per_endpoint);
        assert!(!scenario.cache_metrics.track_latency);
        assert_eq!(scenario.cache_metrics.warmup_requests, 0);
        assert_eq!(scenario.cache_metrics.expected_hit_rate, None);
    }

    #[rstest]
    fn test_to_env_vars_includes_cache_metrics() {
        let scenario = BenchmarkScenario::builder("cache_metrics_env_test")
            .description("Test cache metrics env vars")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig::warm_cache(1000))
            .build();

        let env_vars = scenario.to_env_vars();

        assert!(
            env_vars.contains(&("CACHE_METRICS_ENABLED".to_string(), "1".to_string())),
            "Expected CACHE_METRICS_ENABLED=1"
        );
        assert!(
            env_vars.contains(&("CACHE_METRICS_PER_ENDPOINT".to_string(), "1".to_string())),
            "Expected CACHE_METRICS_PER_ENDPOINT=1"
        );
        assert!(
            env_vars.contains(&("CACHE_METRICS_TRACK_LATENCY".to_string(), "1".to_string())),
            "Expected CACHE_METRICS_TRACK_LATENCY=1"
        );
        assert!(
            env_vars.contains(&("CACHE_WARMUP_REQUESTS".to_string(), "1000".to_string())),
            "Expected CACHE_WARMUP_REQUESTS=1000"
        );
        assert!(
            env_vars.contains(&("EXPECTED_CACHE_HIT_RATE".to_string(), "0.8".to_string())),
            "Expected EXPECTED_CACHE_HIT_RATE=0.8"
        );
    }

    #[rstest]
    fn test_to_env_vars_cache_metrics_disabled() {
        let scenario = BenchmarkScenario::builder("cache_metrics_disabled_test")
            .description("Test cache metrics disabled")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig {
                enabled: false,
                per_endpoint: false,
                track_latency: false,
                warmup_requests: 0,
                expected_hit_rate: None,
            })
            .build();

        let env_vars = scenario.to_env_vars();

        // No cache metrics env vars should be set when disabled
        assert!(
            !env_vars.iter().any(|(k, _)| k == "CACHE_METRICS_ENABLED"),
            "CACHE_METRICS_ENABLED should not be set when disabled"
        );
        assert!(
            !env_vars.iter().any(|(k, _)| k == "CACHE_WARMUP_REQUESTS"),
            "CACHE_WARMUP_REQUESTS should not be set when disabled"
        );
        assert!(
            !env_vars
                .iter()
                .any(|(k, _)| k == "CACHE_METRICS_PER_ENDPOINT"),
            "CACHE_METRICS_PER_ENDPOINT should not be set when disabled"
        );
        assert!(
            !env_vars
                .iter()
                .any(|(k, _)| k == "CACHE_METRICS_TRACK_LATENCY"),
            "CACHE_METRICS_TRACK_LATENCY should not be set when disabled"
        );
        assert!(
            !env_vars.iter().any(|(k, _)| k == "EXPECTED_CACHE_HIT_RATE"),
            "EXPECTED_CACHE_HIT_RATE should not be set when disabled"
        );
    }

    #[rstest]
    fn test_builder_with_cache_metrics() {
        let scenario = BenchmarkScenario::builder("builder_cache_metrics_test")
            .description("Test builder with cache metrics")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig::cold_cache())
            .build();

        assert!(scenario.cache_metrics.enabled);
        assert!(scenario.cache_metrics.per_endpoint);
        assert!(scenario.cache_metrics.track_latency);
        assert_eq!(scenario.cache_metrics.warmup_requests, 0);
        assert_eq!(scenario.cache_metrics.expected_hit_rate, Some(0.0));
    }

    // -------------------------------------------------------------------------
    // Environment and get_cache_environment() Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_with_environment() {
        let yaml = r#"
name: "env_test"
description: "Test scenario with environment"
storage_mode: postgres
cache_mode: redis
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant

environment:
  CACHE_ENABLED: "true"
  CACHE_STRATEGY: "read-through"
  CACHE_TTL_SECS: "60"
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert_eq!(scenario.environment.len(), 3);
        assert_eq!(
            scenario.environment.get("CACHE_ENABLED"),
            Some(&"true".to_string())
        );
        assert_eq!(
            scenario.environment.get("CACHE_STRATEGY"),
            Some(&"read-through".to_string())
        );
        assert_eq!(
            scenario.environment.get("CACHE_TTL_SECS"),
            Some(&"60".to_string())
        );
    }

    #[rstest]
    fn test_scenario_without_environment() {
        let yaml = r#"
name: "no_env_test"
description: "Test scenario without environment"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: read_heavy
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;

        let scenario = BenchmarkScenario::from_yaml(yaml).unwrap();

        assert!(scenario.environment.is_empty());
    }

    #[rstest]
    fn test_get_cache_environment_with_env_and_cache_metrics() {
        let mut env = HashMap::new();
        env.insert("CACHE_ENABLED".to_string(), "true".to_string());
        env.insert("CACHE_STRATEGY".to_string(), "read-through".to_string());
        env.insert("CACHE_TTL_SECS".to_string(), "60".to_string());

        let scenario = BenchmarkScenario::builder("cache_env_test")
            .description("Test get_cache_environment")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig::warm_cache(1000))
            .environment(env)
            .build();

        let cache_env = scenario.get_cache_environment();

        // Environment section values
        assert_eq!(cache_env.get("CACHE_ENABLED"), Some(&"true".to_string()));
        assert_eq!(
            cache_env.get("CACHE_STRATEGY"),
            Some(&"read-through".to_string())
        );
        assert_eq!(cache_env.get("CACHE_TTL_SECS"), Some(&"60".to_string()));

        // cache_metrics values
        assert_eq!(
            cache_env.get("CACHE_METRICS_ENABLED"),
            Some(&"1".to_string())
        );
        assert_eq!(
            cache_env.get("CACHE_WARMUP_REQUESTS"),
            Some(&"1000".to_string())
        );
        assert_eq!(
            cache_env.get("EXPECTED_CACHE_HIT_RATE"),
            Some(&"0.8".to_string())
        );
        // per_endpoint and track_latency are set to true by warm_cache()
        assert_eq!(
            cache_env.get("CACHE_METRICS_PER_ENDPOINT"),
            Some(&"1".to_string())
        );
        assert_eq!(
            cache_env.get("CACHE_METRICS_TRACK_LATENCY"),
            Some(&"1".to_string())
        );
    }

    #[rstest]
    fn test_get_cache_environment_env_takes_precedence() {
        let mut env = HashMap::new();
        // Environment section overrides cache_metrics generated values
        env.insert("CACHE_WARMUP_REQUESTS".to_string(), "2000".to_string());

        let scenario = BenchmarkScenario::builder("env_precedence_test")
            .description("Test environment takes precedence")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig::warm_cache(1000)) // warmup_requests = 1000
            .environment(env)
            .build();

        let cache_env = scenario.get_cache_environment();

        // Environment section value (2000) takes precedence over cache_metrics (1000)
        assert_eq!(
            cache_env.get("CACHE_WARMUP_REQUESTS"),
            Some(&"2000".to_string())
        );
    }

    #[rstest]
    fn test_builder_with_environment() {
        let mut env = HashMap::new();
        env.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());

        let scenario = BenchmarkScenario::builder("builder_env_test")
            .description("Test builder with environment")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .environment(env)
            .build();

        assert_eq!(scenario.environment.len(), 1);
        assert_eq!(
            scenario.environment.get("CUSTOM_VAR"),
            Some(&"custom_value".to_string())
        );
    }

    // -------------------------------------------------------------------------
    // ErrorConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_error_config_default() {
        let config = ErrorConfig::default();
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.connect_timeout_ms, 5000);
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.retry_delay_ms, 1000);
        assert_eq!(config.expected_error_rate, None);
        assert!(!config.fail_on_error_threshold);
        assert_eq!(config.inject_error_rate, None);
    }

    #[rstest]
    fn test_error_config_stress_test() {
        let config = ErrorConfig::stress_test();
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.connect_timeout_ms, 1000);
        assert_eq!(config.max_retries, 0);
        assert_eq!(config.retry_delay_ms, 100);
        assert_eq!(config.expected_error_rate, Some(0.1));
        assert!(config.fail_on_error_threshold);
        assert_eq!(config.inject_error_rate, None);
    }

    #[rstest]
    fn test_error_config_chaos_test() {
        let config = ErrorConfig::chaos_test(0.1);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.connect_timeout_ms, 2000);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.retry_delay_ms, 500);
        // Use approximate comparison for floating point (0.1 * 1.5 = 0.15)
        let expected_rate = config.expected_error_rate.unwrap();
        assert!(
            (expected_rate - 0.15).abs() < f64::EPSILON * 10.0,
            "expected_error_rate should be approximately 0.15, got {expected_rate}"
        );
        assert!(config.fail_on_error_threshold);
        assert_eq!(config.inject_error_rate, Some(0.1));
    }

    #[rstest]
    fn test_error_config_is_configured() {
        let default_config = ErrorConfig::default();
        assert!(!default_config.is_configured());

        let config_with_retries = ErrorConfig {
            max_retries: 1,
            ..Default::default()
        };
        assert!(config_with_retries.is_configured());

        let config_with_error_rate = ErrorConfig {
            expected_error_rate: Some(0.05),
            ..Default::default()
        };
        assert!(config_with_error_rate.is_configured());

        let config_with_injection = ErrorConfig {
            inject_error_rate: Some(0.1),
            ..Default::default()
        };
        assert!(config_with_injection.is_configured());
    }

    #[rstest]
    fn test_error_config_serde() {
        let yaml = r"
timeout_ms: 5000
connect_timeout_ms: 1000
max_retries: 3
retry_delay_ms: 500
expected_error_rate: 0.05
fail_on_error_threshold: true
inject_error_rate: 0.1
";
        let config: ErrorConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.connect_timeout_ms, 1000);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 500);
        assert_eq!(config.expected_error_rate, Some(0.05));
        assert!(config.fail_on_error_threshold);
        assert_eq!(config.inject_error_rate, Some(0.1));
    }

    #[rstest]
    fn test_error_config_serde_defaults() {
        let yaml = r"
max_retries: 2
";
        let config: ErrorConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.timeout_ms, 30000); // default
        assert_eq!(config.connect_timeout_ms, 5000); // default
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.retry_delay_ms, 1000); // default
        assert_eq!(config.expected_error_rate, None); // default
        assert!(!config.fail_on_error_threshold); // default
        assert_eq!(config.inject_error_rate, None); // default
    }

    #[rstest]
    fn test_to_env_vars_error_config() {
        let scenario = BenchmarkScenario::builder("error_config_test")
            .description("Test error config environment variables")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .error_config(ErrorConfig {
                timeout_ms: 5000,
                connect_timeout_ms: 1000,
                max_retries: 2,
                retry_delay_ms: 500,
                expected_error_rate: Some(0.05),
                fail_on_error_threshold: true,
                inject_error_rate: Some(0.1),
            })
            .build();

        let env_vars = scenario.to_env_vars();

        assert!(
            env_vars.contains(&("REQUEST_TIMEOUT_MS".to_string(), "5000".to_string())),
            "Expected REQUEST_TIMEOUT_MS=5000"
        );
        assert!(
            env_vars.contains(&("CONNECT_TIMEOUT_MS".to_string(), "1000".to_string())),
            "Expected CONNECT_TIMEOUT_MS=1000"
        );
        assert!(
            env_vars.contains(&("MAX_RETRIES".to_string(), "2".to_string())),
            "Expected MAX_RETRIES=2"
        );
        assert!(
            env_vars.contains(&("RETRY_DELAY_MS".to_string(), "500".to_string())),
            "Expected RETRY_DELAY_MS=500"
        );
        assert!(
            env_vars.contains(&("EXPECTED_ERROR_RATE".to_string(), "0.05".to_string())),
            "Expected EXPECTED_ERROR_RATE=0.05"
        );
        assert!(
            env_vars.contains(&("FAIL_ON_ERROR_THRESHOLD".to_string(), "1".to_string())),
            "Expected FAIL_ON_ERROR_THRESHOLD=1"
        );
        assert!(
            env_vars.contains(&("INJECT_ERROR_RATE".to_string(), "0.1".to_string())),
            "Expected INJECT_ERROR_RATE=0.1"
        );
    }

    #[rstest]
    fn test_to_env_vars_error_config_default() {
        let scenario = BenchmarkScenario::builder("error_config_default_test")
            .description("Test error config default environment variables")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let env_vars = scenario.to_env_vars();

        // Default values should still be present
        assert!(
            env_vars.contains(&("REQUEST_TIMEOUT_MS".to_string(), "30000".to_string())),
            "Expected REQUEST_TIMEOUT_MS=30000 (default)"
        );
        assert!(
            env_vars.contains(&("CONNECT_TIMEOUT_MS".to_string(), "5000".to_string())),
            "Expected CONNECT_TIMEOUT_MS=5000 (default)"
        );
        assert!(
            env_vars.contains(&("MAX_RETRIES".to_string(), "0".to_string())),
            "Expected MAX_RETRIES=0 (default)"
        );
        assert!(
            env_vars.contains(&("RETRY_DELAY_MS".to_string(), "1000".to_string())),
            "Expected RETRY_DELAY_MS=1000 (default)"
        );

        // Optional fields should not be present when None/false
        assert!(
            !env_vars.iter().any(|(k, _)| k == "EXPECTED_ERROR_RATE"),
            "EXPECTED_ERROR_RATE should not be set when None"
        );
        assert!(
            !env_vars.iter().any(|(k, _)| k == "FAIL_ON_ERROR_THRESHOLD"),
            "FAIL_ON_ERROR_THRESHOLD should not be set when false"
        );
        assert!(
            !env_vars.iter().any(|(k, _)| k == "INJECT_ERROR_RATE"),
            "INJECT_ERROR_RATE should not be set when None"
        );
    }

    #[rstest]
    fn test_builder_with_error_config() {
        let scenario = BenchmarkScenario::builder("builder_error_config_test")
            .description("Test builder with error config")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .error_config(ErrorConfig::stress_test())
            .build();

        assert_eq!(scenario.error_config.timeout_ms, 5000);
        assert_eq!(scenario.error_config.connect_timeout_ms, 1000);
        assert_eq!(scenario.error_config.max_retries, 0);
        assert_eq!(scenario.error_config.retry_delay_ms, 100);
        assert_eq!(scenario.error_config.expected_error_rate, Some(0.1));
        assert!(scenario.error_config.fail_on_error_threshold);
    }

    #[rstest]
    fn test_scenario_yaml_with_error_config() {
        let yaml = r#"
name: "error_test"
description: "Test scenario with error config"
storage_mode: postgres
cache_mode: redis
load_pattern: mixed
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
error_config:
  timeout_ms: 5000
  connect_timeout_ms: 1000
  max_retries: 2
  retry_delay_ms: 500
  expected_error_rate: 0.05
  fail_on_error_threshold: true
"#;
        let scenario: BenchmarkScenario = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(scenario.name, "error_test");
        assert_eq!(scenario.error_config.timeout_ms, 5000);
        assert_eq!(scenario.error_config.connect_timeout_ms, 1000);
        assert_eq!(scenario.error_config.max_retries, 2);
        assert_eq!(scenario.error_config.retry_delay_ms, 500);
        assert_eq!(scenario.error_config.expected_error_rate, Some(0.05));
        assert!(scenario.error_config.fail_on_error_threshold);
        assert_eq!(scenario.error_config.inject_error_rate, None);
    }

    #[rstest]
    fn test_scenario_yaml_without_error_config() {
        let yaml = r#"
name: "default_error_test"
description: "Test scenario without error config"
storage_mode: postgres
cache_mode: redis
load_pattern: mixed
cache_state: warm
data_scale: medium
payload_variant: standard
rps_profile: constant
"#;
        let scenario: BenchmarkScenario = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(scenario.name, "default_error_test");
        // Should use default error config
        assert_eq!(scenario.error_config.timeout_ms, 30000);
        assert_eq!(scenario.error_config.connect_timeout_ms, 5000);
        assert_eq!(scenario.error_config.max_retries, 0);
        assert!(!scenario.error_config.fail_on_error_threshold);
    }

    #[rstest]
    fn test_error_resilience_scenario_file() {
        let scenario = BenchmarkScenario::from_file(
            "/Users/lihs/workspace/lambars-api-benchmark/benches/api/benchmarks/scenarios/error_resilience_test.yaml",
        )
        .expect("Failed to load error_resilience_test.yaml");

        assert_eq!(scenario.name, "error_resilience_test");
        assert_eq!(scenario.error_config.timeout_ms, 5000);
        assert_eq!(scenario.error_config.connect_timeout_ms, 1000);
        assert_eq!(scenario.error_config.max_retries, 2);
        assert_eq!(scenario.error_config.retry_delay_ms, 500);
        assert_eq!(scenario.error_config.expected_error_rate, Some(0.05));
        assert!(scenario.error_config.fail_on_error_threshold);
        assert_eq!(scenario.error_config.inject_error_rate, None);
    }

    #[rstest]
    fn test_chaos_test_scenario_file() {
        let scenario = BenchmarkScenario::from_file(
            "/Users/lihs/workspace/lambars-api-benchmark/benches/api/benchmarks/scenarios/chaos_test.yaml",
        )
        .expect("Failed to load chaos_test.yaml");

        assert_eq!(scenario.name, "chaos_test");
        assert_eq!(scenario.error_config.timeout_ms, 10000);
        assert_eq!(scenario.error_config.connect_timeout_ms, 2000);
        assert_eq!(scenario.error_config.max_retries, 3);
        assert_eq!(scenario.error_config.retry_delay_ms, 500);
        assert_eq!(scenario.error_config.expected_error_rate, Some(0.15));
        assert!(scenario.error_config.fail_on_error_threshold);
        assert_eq!(scenario.error_config.inject_error_rate, Some(0.1));
    }

    // -------------------------------------------------------------------------
    // PartialScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_partial_scenario_default() {
        let partial = PartialScenario::default();

        assert!(partial.storage_mode.is_none());
        assert!(partial.cache_mode.is_none());
        assert!(partial.load_pattern.is_none());
        assert!(partial.cache_state.is_none());
        assert!(partial.data_scale.is_none());
        assert!(partial.payload_variant.is_none());
        assert!(partial.rps_profile.is_none());
        assert!(partial.contention_level.is_none());
        assert!(partial.duration_seconds.is_none());
        assert!(partial.connections.is_none());
        assert!(partial.threads.is_none());
        assert!(partial.warmup_seconds.is_none());
        assert!(partial.target_rps.is_none());
        assert!(partial.profiling.is_none());
        assert!(partial.data_scale_config.is_none());
        assert!(partial.cache_metrics.is_none());
        assert!(partial.error_config.is_none());
        assert!(partial.concurrency.is_none());
        assert!(partial.thresholds.is_none());
    }

    #[rstest]
    fn test_partial_scenario_merge_other_takes_precedence() {
        let base = PartialScenario {
            storage_mode: Some(StorageMode::InMemory),
            cache_mode: Some(CacheMode::InMemory),
            duration_seconds: Some(60),
            connections: Some(10),
            ..Default::default()
        };

        let override_partial = PartialScenario {
            storage_mode: Some(StorageMode::Postgres),
            duration_seconds: Some(120),
            ..Default::default()
        };

        let merged = base.merge(&override_partial);

        // Override values take precedence
        assert_eq!(merged.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(merged.duration_seconds, Some(120));
        // Base values are preserved when not overridden
        assert_eq!(merged.cache_mode, Some(CacheMode::InMemory));
        assert_eq!(merged.connections, Some(10));
        // Still None values
        assert!(merged.load_pattern.is_none());
    }

    #[rstest]
    fn test_partial_scenario_merge_none_does_not_override() {
        let base = PartialScenario {
            storage_mode: Some(StorageMode::Postgres),
            threads: Some(8),
            ..Default::default()
        };

        let empty_override = PartialScenario::default();

        let merged = base.merge(&empty_override);

        // Base values are preserved when override is None
        assert_eq!(merged.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(merged.threads, Some(8));
    }

    #[rstest]
    fn test_partial_scenario_apply_to() {
        let mut scenario = BenchmarkScenario::builder("test")
            .description("Test scenario")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Small)
            .payload_variant(PayloadVariant::Minimal)
            .rps_profile(RpsProfile::Constant)
            .duration_seconds(60)
            .connections(10)
            .build();

        let partial = PartialScenario {
            storage_mode: Some(StorageMode::Postgres),
            cache_mode: Some(CacheMode::Redis),
            duration_seconds: Some(120),
            connections: Some(50),
            ..Default::default()
        };

        partial.apply_to(&mut scenario);

        // Applied values
        assert_eq!(scenario.storage_mode, StorageMode::Postgres);
        assert_eq!(scenario.cache_mode, CacheMode::Redis);
        assert_eq!(scenario.duration_seconds, 120);
        assert_eq!(scenario.connections, 50);
        // Unchanged values
        assert_eq!(scenario.load_pattern, LoadPattern::ReadHeavy);
        assert_eq!(scenario.cache_state, CacheState::Warm);
        assert_eq!(scenario.data_scale, DataScale::Small);
    }

    #[rstest]
    fn test_partial_scenario_apply_to_with_complex_types() {
        let mut scenario = BenchmarkScenario::builder("test")
            .description("Test scenario")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let partial = PartialScenario {
            profiling: Some(ProfilingConfig::with_flamegraph()),
            error_config: Some(ErrorConfig::stress_test()),
            concurrency: Some(ConcurrencyConfig::large_pool()),
            thresholds: Some(Thresholds {
                max_error_rate: 0.05,
                p99_latency_ms: 200,
                min_rps_achieved: 1000,
            }),
            ..Default::default()
        };

        partial.apply_to(&mut scenario);

        assert!(scenario.profiling.enable_flamegraph);
        assert_eq!(scenario.error_config.timeout_ms, 5000);
        assert!(scenario.concurrency.is_some());
        assert_eq!(scenario.concurrency.as_ref().unwrap().worker_threads, 8);
        assert!(scenario.thresholds.is_some());
        assert_eq!(scenario.thresholds.as_ref().unwrap().min_rps_achieved, 1000);
    }

    #[rstest]
    fn test_partial_scenario_serde() {
        let yaml = r"
storage_mode: postgres
cache_mode: redis
load_pattern: mixed
duration_seconds: 120
connections: 100
";
        let partial: PartialScenario = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(partial.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(partial.cache_mode, Some(CacheMode::Redis));
        assert_eq!(partial.load_pattern, Some(LoadPattern::Mixed));
        assert_eq!(partial.duration_seconds, Some(120));
        assert_eq!(partial.connections, Some(100));
        assert!(partial.cache_state.is_none());
    }

    #[rstest]
    fn test_partial_scenario_serde_rejects_unknown_fields() {
        let yaml = r"
storage_mode: postgres
unknown_field: value
";
        let result: Result<PartialScenario, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // ScenarioTemplate Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_template_new() {
        let template = ScenarioTemplate::new("high_load", "High load template");

        assert_eq!(template.name, "high_load");
        assert_eq!(template.description, "High load template");
        assert_eq!(template.base, PartialScenario::default());
    }

    #[rstest]
    fn test_scenario_template_serde() {
        let yaml = r#"
name: "high_load"
description: "Template for high load scenarios"
storage_mode: postgres
cache_mode: redis
load_pattern: mixed
cache_state: warm
data_scale: large
contention_level: high
duration_seconds: 120
connections: 100
threads: 16
target_rps: 5000
"#;
        let template: ScenarioTemplate = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(template.name, "high_load");
        assert_eq!(template.description, "Template for high load scenarios");
        assert_eq!(template.base.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(template.base.cache_mode, Some(CacheMode::Redis));
        assert_eq!(template.base.load_pattern, Some(LoadPattern::Mixed));
        assert_eq!(template.base.cache_state, Some(CacheState::Warm));
        assert_eq!(template.base.data_scale, Some(DataScale::Large));
        assert_eq!(template.base.contention_level, Some(ContentionLevel::High));
        assert_eq!(template.base.duration_seconds, Some(120));
        assert_eq!(template.base.connections, Some(100));
        assert_eq!(template.base.threads, Some(16));
        assert_eq!(template.base.target_rps, Some(5000));
    }

    #[rstest]
    fn test_scenario_template_yaml_roundtrip() {
        let template = ScenarioTemplate {
            name: "test_template".to_string(),
            description: "Test template".to_string(),
            base: PartialScenario {
                storage_mode: Some(StorageMode::Postgres),
                cache_mode: Some(CacheMode::Redis),
                duration_seconds: Some(120),
                ..Default::default()
            },
        };

        let yaml = template.to_yaml().unwrap();
        let parsed: ScenarioTemplate = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(parsed.name, "test_template");
        assert_eq!(parsed.description, "Test template");
        assert_eq!(parsed.base.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(parsed.base.cache_mode, Some(CacheMode::Redis));
        assert_eq!(parsed.base.duration_seconds, Some(120));
    }

    // -------------------------------------------------------------------------
    // ScenarioValidation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_validation_new() {
        let validation = ScenarioValidation::new();

        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert!(validation.warnings.is_empty());
    }

    #[rstest]
    fn test_scenario_validation_add_error() {
        let mut validation = ScenarioValidation::new();

        validation.add_error("Test error");

        assert!(!validation.is_valid);
        assert_eq!(validation.errors.len(), 1);
        assert_eq!(validation.errors[0], "Test error");
    }

    #[rstest]
    fn test_scenario_validation_add_warning() {
        let mut validation = ScenarioValidation::new();

        validation.add_warning("Test warning");

        // Warnings don't invalidate
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
        assert_eq!(validation.warnings.len(), 1);
        assert_eq!(validation.warnings[0], "Test warning");
    }

    #[rstest]
    fn test_scenario_validate_valid() {
        let scenario = BenchmarkScenario::builder("valid_test")
            .description("A valid test scenario")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }

    #[rstest]
    fn test_scenario_validate_empty_name() {
        let scenario = BenchmarkScenario {
            name: String::new(),
            description: "Has description".to_string(),
            ..Default::default()
        };

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.contains("name")));
    }

    #[rstest]
    fn test_scenario_validate_empty_description() {
        let scenario = BenchmarkScenario {
            name: "has_name".to_string(),
            description: String::new(),
            ..Default::default()
        };

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(validation.errors.iter().any(|e| e.contains("description")));
    }

    #[rstest]
    fn test_scenario_validate_threads_greater_than_connections() {
        let scenario = BenchmarkScenario::builder("thread_test")
            .description("Test threads vs connections")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .connections(5)
            .threads(10) // More threads than connections
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid); // Still valid, just has warning
        assert!(validation.warnings.iter().any(|w| w.contains("threads")));
    }

    #[rstest]
    fn test_scenario_validate_cold_cache_with_warmup() {
        let scenario = BenchmarkScenario::builder("cache_test")
            .description("Test cold cache with warmup")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Cold)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig {
                enabled: true,
                per_endpoint: true,
                track_latency: true,
                warmup_requests: 1000, // Has warmup
                expected_hit_rate: None,
            })
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid);
        assert!(validation.warnings.iter().any(|w| w.contains("warmup")));
    }

    #[rstest]
    fn test_scenario_validate_large_scale_short_duration() {
        let scenario = BenchmarkScenario::builder("scale_test")
            .description("Test large scale with short duration")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .duration_seconds(30) // Less than 60
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid);
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("Large data scale"))
        );
    }

    #[rstest]
    fn test_scenario_validate_error_injection_without_expected_rate() {
        let scenario = BenchmarkScenario::builder("error_test")
            .description("Test error injection config")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .error_config(ErrorConfig {
                inject_error_rate: Some(0.1), // Has injection
                expected_error_rate: None,    // But no expected rate
                ..Default::default()
            })
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid);
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("Error injection"))
        );
    }

    // -------------------------------------------------------------------------
    // ExtendableScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_extendable_scenario_resolve_without_template() {
        let extendable = ExtendableScenario {
            extends: None,
            scenario: BenchmarkScenario::builder("standalone")
                .description("Standalone scenario")
                .storage_mode(StorageMode::InMemory)
                .cache_mode(CacheMode::InMemory)
                .load_pattern(LoadPattern::ReadHeavy)
                .cache_state(CacheState::Warm)
                .data_scale(DataScale::Medium)
                .payload_variant(PayloadVariant::Standard)
                .rps_profile(RpsProfile::Constant)
                .build(),
        };

        let templates = std::collections::HashMap::new();
        let resolved = extendable.resolve(&templates).unwrap();

        assert_eq!(resolved.name, "standalone");
        assert_eq!(resolved.storage_mode, StorageMode::InMemory);
    }

    #[rstest]
    fn test_extendable_scenario_resolve_with_template() {
        let template = ScenarioTemplate {
            name: "high_load".to_string(),
            description: "High load template".to_string(),
            base: PartialScenario {
                storage_mode: Some(StorageMode::Postgres),
                cache_mode: Some(CacheMode::Redis),
                duration_seconds: Some(120),
                connections: Some(100),
                threads: Some(16),
                ..Default::default()
            },
        };

        let mut templates = std::collections::HashMap::new();
        templates.insert("high_load".to_string(), template);

        // Note: All scenario values override template values, including defaults
        let extendable = ExtendableScenario {
            extends: Some("high_load".to_string()),
            scenario: BenchmarkScenario::builder("production_test")
                .description("Production test")
                .storage_mode(StorageMode::InMemory)
                .cache_mode(CacheMode::InMemory)
                .load_pattern(LoadPattern::Mixed)
                .cache_state(CacheState::Warm)
                .data_scale(DataScale::Large)
                .payload_variant(PayloadVariant::Complex)
                .rps_profile(RpsProfile::Burst)
                .duration_seconds(120) // Explicitly set to match template
                .connections(100) // Explicitly set to match template
                .threads(16) // Explicitly set to match template
                .build(),
        };

        let resolved = extendable.resolve(&templates).unwrap();

        // Scenario values override template values
        assert_eq!(resolved.storage_mode, StorageMode::InMemory);
        assert_eq!(resolved.cache_mode, CacheMode::InMemory);
        assert_eq!(resolved.duration_seconds, 120);
        assert_eq!(resolved.connections, 100);
        assert_eq!(resolved.threads, 16);
        // Scenario's own values are preserved
        assert_eq!(resolved.load_pattern, LoadPattern::Mixed);
        assert_eq!(resolved.data_scale, DataScale::Large);
    }

    #[rstest]
    fn test_extendable_scenario_resolve_template_not_found() {
        let extendable = ExtendableScenario {
            extends: Some("nonexistent".to_string()),
            scenario: BenchmarkScenario::builder("test")
                .description("Test")
                .storage_mode(StorageMode::InMemory)
                .cache_mode(CacheMode::InMemory)
                .load_pattern(LoadPattern::ReadHeavy)
                .cache_state(CacheState::Warm)
                .data_scale(DataScale::Medium)
                .payload_variant(PayloadVariant::Standard)
                .rps_profile(RpsProfile::Constant)
                .build(),
        };

        let templates = std::collections::HashMap::new();
        let result = extendable.resolve(&templates);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ScenarioError::TemplateNotFound(_)));
    }

    #[rstest]
    fn test_extendable_scenario_serde() {
        let yaml = r#"
extends: "high_load"
name: "production_test"
description: "Production test"
storage_mode: in_memory
cache_mode: in_memory
load_pattern: mixed
cache_state: warm
data_scale: large
payload_variant: complex
rps_profile: burst
"#;
        let extendable: ExtendableScenario = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(extendable.extends, Some("high_load".to_string()));
        assert_eq!(extendable.scenario.name, "production_test");
        assert_eq!(extendable.scenario.load_pattern, LoadPattern::Mixed);
    }

    // -------------------------------------------------------------------------
    // ScenarioRegistry Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_registry_new() {
        let registry = ScenarioRegistry::new();

        assert!(registry.list_templates().is_empty());
        assert!(registry.list_scenarios().is_empty());
    }

    #[rstest]
    fn test_scenario_registry_register_template() {
        let mut registry = ScenarioRegistry::new();

        let template = ScenarioTemplate::new("test_template", "Test template");
        registry.register_template(template);

        assert_eq!(registry.list_templates(), vec!["test_template"]);
        assert!(registry.get_template("test_template").is_some());
    }

    #[rstest]
    fn test_scenario_registry_register_scenario() {
        let mut registry = ScenarioRegistry::new();

        let scenario = BenchmarkScenario::builder("test_scenario")
            .description("Test scenario")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .build();

        registry.register_scenario(scenario);

        assert_eq!(registry.list_scenarios(), vec!["test_scenario"]);
        assert!(registry.get_scenario("test_scenario").is_some());
    }

    #[rstest]
    fn test_scenario_registry_resolve_scenario() {
        let mut registry = ScenarioRegistry::new();

        let template = ScenarioTemplate {
            name: "base".to_string(),
            description: "Base template".to_string(),
            base: PartialScenario {
                storage_mode: Some(StorageMode::Postgres),
                duration_seconds: Some(120),
                ..Default::default()
            },
        };
        registry.register_template(template);

        // Note: All scenario values override template values, including defaults
        let extendable = ExtendableScenario {
            extends: Some("base".to_string()),
            scenario: BenchmarkScenario::builder("derived")
                .description("Derived scenario")
                .storage_mode(StorageMode::InMemory)
                .cache_mode(CacheMode::Redis)
                .load_pattern(LoadPattern::Mixed)
                .cache_state(CacheState::Warm)
                .data_scale(DataScale::Large)
                .payload_variant(PayloadVariant::Complex)
                .rps_profile(RpsProfile::Burst)
                .duration_seconds(120) // Explicitly set to match template
                .build(),
        };

        let resolved = registry.resolve_scenario(&extendable).unwrap();

        // Scenario values override template values
        assert_eq!(resolved.storage_mode, StorageMode::InMemory);
        assert_eq!(resolved.cache_mode, CacheMode::Redis);
        assert_eq!(resolved.duration_seconds, 120);
        // Scenario name and description are preserved
        assert_eq!(resolved.name, "derived");
        assert_eq!(resolved.description, "Derived scenario");
    }

    #[rstest]
    fn test_scenario_registry_templates_accessor() {
        let mut registry = ScenarioRegistry::new();

        registry.register_template(ScenarioTemplate::new("a", "Template A"));
        registry.register_template(ScenarioTemplate::new("b", "Template B"));

        let templates = registry.templates();
        assert_eq!(templates.len(), 2);
        assert!(templates.contains_key("a"));
        assert!(templates.contains_key("b"));
    }

    // -------------------------------------------------------------------------
    // ScenarioError TemplateNotFound Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_error_template_not_found_display() {
        let error = ScenarioError::TemplateNotFound("missing_template".to_string());
        assert!(error.to_string().contains("missing_template"));
        assert!(error.to_string().contains("Template not found"));
    }

    // -------------------------------------------------------------------------
    // ScenarioRegistry::load_templates_from_directory Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_registry_load_templates_from_directory() {
        let directory = tempfile::tempdir().unwrap();

        // Create template files
        let template1_yaml = r#"
name: "test_template_1"
description: "Test template 1"
storage_mode: postgres
cache_mode: redis
"#;
        let path1 = directory.path().join("test_template_1.yaml");
        std::fs::write(&path1, template1_yaml).unwrap();

        let template2_yaml = r#"
name: "test_template_2"
description: "Test template 2"
storage_mode: in_memory
cache_mode: in_memory
"#;
        let path2 = directory.path().join("test_template_2.yml");
        std::fs::write(&path2, template2_yaml).unwrap();

        // Load templates
        let mut registry = ScenarioRegistry::new();
        registry
            .load_templates_from_directory(directory.path())
            .unwrap();

        let templates = registry.list_templates();
        assert_eq!(templates.len(), 2);
        assert!(registry.get_template("test_template_1").is_some());
        assert!(registry.get_template("test_template_2").is_some());

        // Verify template contents
        let template1 = registry.get_template("test_template_1").unwrap();
        assert_eq!(template1.description, "Test template 1");
        assert_eq!(template1.base.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(template1.base.cache_mode, Some(CacheMode::Redis));
    }

    #[rstest]
    fn test_scenario_registry_load_templates_from_directory_ignores_non_yaml_files() {
        let directory = tempfile::tempdir().unwrap();

        // Create a valid template file
        let template_yaml = r#"
name: "valid_template"
description: "Valid template"
storage_mode: postgres
"#;
        let yaml_path = directory.path().join("valid.yaml");
        std::fs::write(&yaml_path, template_yaml).unwrap();

        // Create non-YAML files that should be ignored
        let txt_path = directory.path().join("not_yaml.txt");
        std::fs::write(&txt_path, "this is not yaml").unwrap();

        let json_path = directory.path().join("also_not_yaml.json");
        std::fs::write(&json_path, r#"{"name": "json"}"#).unwrap();

        // Load templates
        let mut registry = ScenarioRegistry::new();
        registry
            .load_templates_from_directory(directory.path())
            .unwrap();

        // Only the YAML file should be loaded
        assert_eq!(registry.list_templates().len(), 1);
        assert!(registry.get_template("valid_template").is_some());
    }

    #[rstest]
    fn test_scenario_registry_load_templates_handles_invalid_yaml() {
        let directory = tempfile::tempdir().unwrap();

        // Create an invalid YAML file
        let invalid_yaml_path = directory.path().join("invalid.yaml");
        std::fs::write(&invalid_yaml_path, "this is: not: valid: yaml:").unwrap();

        // Load templates should fail
        let mut registry = ScenarioRegistry::new();
        let result = registry.load_templates_from_directory(directory.path());

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScenarioError::YamlParse(_)));
    }

    #[rstest]
    fn test_scenario_registry_load_templates_from_nonexistent_directory() {
        let mut registry = ScenarioRegistry::new();
        let result =
            registry.load_templates_from_directory(std::path::Path::new("/nonexistent/directory"));

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScenarioError::FileRead(_)));
    }

    #[rstest]
    fn test_scenario_registry_load_templates_from_empty_directory() {
        let directory = tempfile::tempdir().unwrap();

        let mut registry = ScenarioRegistry::new();
        registry
            .load_templates_from_directory(directory.path())
            .unwrap();

        assert!(registry.list_templates().is_empty());
    }

    // -------------------------------------------------------------------------
    // Extended Validation Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_scenario_validate_expected_error_rate_out_of_range() {
        let scenario = BenchmarkScenario::builder("error_rate_test")
            .description("Test error rate validation")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .error_config(ErrorConfig {
                expected_error_rate: Some(1.5), // Out of range
                ..Default::default()
            })
            .build();

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|e| e.contains("expected_error_rate"))
        );
    }

    #[rstest]
    fn test_scenario_validate_inject_error_rate_out_of_range() {
        let scenario = BenchmarkScenario::builder("inject_error_test")
            .description("Test inject error rate validation")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .error_config(ErrorConfig {
                inject_error_rate: Some(-0.1), // Out of range
                ..Default::default()
            })
            .build();

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|e| e.contains("inject_error_rate"))
        );
    }

    #[rstest]
    fn test_scenario_validate_expected_hit_rate_out_of_range() {
        let scenario = BenchmarkScenario::builder("hit_rate_test")
            .description("Test expected hit rate validation")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .cache_metrics(CacheMetricsConfig {
                enabled: true,
                expected_hit_rate: Some(2.0), // Out of range
                ..Default::default()
            })
            .build();

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|e| e.contains("expected_hit_rate"))
        );
    }

    #[rstest]
    fn test_scenario_validate_warmup_greater_than_duration() {
        let scenario = BenchmarkScenario::builder("warmup_test")
            .description("Test warmup vs duration validation")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .duration_seconds(30)
            .warmup_seconds(60) // Warmup > Duration
            .build();

        let validation = scenario.validate();

        assert!(!validation.is_valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|e| e.contains("warmup_seconds"))
        );
    }

    #[rstest]
    fn test_scenario_validate_high_rps_per_connection() {
        let scenario = BenchmarkScenario::builder("high_rps_test")
            .description("Test high RPS per connection warning")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .target_rps(10000)
            .connections(5) // 2000 RPS per connection
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid); // Still valid, just warning
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("RPS per connection"))
        );
    }

    #[rstest]
    fn test_scenario_validate_max_connections_less_than_connections() {
        let scenario = BenchmarkScenario::builder("max_conn_test")
            .description("Test max connections validation")
            .storage_mode(StorageMode::InMemory)
            .cache_mode(CacheMode::InMemory)
            .load_pattern(LoadPattern::ReadHeavy)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Medium)
            .payload_variant(PayloadVariant::Standard)
            .rps_profile(RpsProfile::Constant)
            .connections(100)
            .concurrency(ConcurrencyConfig {
                max_connections: 50, // Less than connections
                ..Default::default()
            })
            .build();

        let validation = scenario.validate();

        assert!(validation.is_valid); // Still valid, just warning
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("max_connections"))
        );
    }

    // -------------------------------------------------------------------------
    // PartialScenario From BenchmarkScenario Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_partial_scenario_from_benchmark_scenario() {
        let scenario = BenchmarkScenario::builder("test")
            .description("Test scenario")
            .storage_mode(StorageMode::Postgres)
            .cache_mode(CacheMode::Redis)
            .load_pattern(LoadPattern::Mixed)
            .cache_state(CacheState::Warm)
            .data_scale(DataScale::Large)
            .payload_variant(PayloadVariant::Complex)
            .rps_profile(RpsProfile::Burst)
            .duration_seconds(120)
            .connections(100)
            .threads(16)
            .build();

        let partial = PartialScenario::from(&scenario);

        assert_eq!(partial.storage_mode, Some(StorageMode::Postgres));
        assert_eq!(partial.cache_mode, Some(CacheMode::Redis));
        assert_eq!(partial.load_pattern, Some(LoadPattern::Mixed));
        assert_eq!(partial.cache_state, Some(CacheState::Warm));
        assert_eq!(partial.data_scale, Some(DataScale::Large));
        assert_eq!(partial.payload_variant, Some(PayloadVariant::Complex));
        assert_eq!(partial.rps_profile, Some(RpsProfile::Burst));
        assert_eq!(partial.duration_seconds, Some(120));
        assert_eq!(partial.connections, Some(100));
        assert_eq!(partial.threads, Some(16));
    }
}
