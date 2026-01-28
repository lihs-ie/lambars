//! Fail injection mechanism for external data sources.
//!
//! This module provides fail injection capabilities for testing and benchmarking:
//!
//! - **Failure rate**: Probability of injecting a failure after successful I/O
//! - **Delay injection**: Add artificial delays to simulate network latency
//! - **Timeout simulation**: Simulate timeout errors
//!
//! # Design Principles
//!
//! - **I/O boundary isolation**: All non-deterministic operations (RNG, time) are
//!   confined within `AsyncIO` closures
//! - **External RNG injection**: RNG providers are injected externally, enabling
//!   deterministic behavior for tests and benchmarks
//! - **Post-I/O application**: Fail injection is applied after real I/O succeeds,
//!   never skipping actual network calls

use std::time::Duration;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use siphasher::sip::SipHasher24;
use std::hash::Hasher;
use thiserror::Error;

use super::external::ExternalError;

// =============================================================================
// Fail Injection Config
// =============================================================================

/// Configuration for fail injection.
///
/// All values are validated at construction time to ensure they are within
/// valid ranges.
#[derive(Debug, Clone)]
pub struct FailInjectionConfig {
    /// Probability of injecting a failure (0.0 - 1.0).
    pub failure_rate: f64,
    /// Minimum delay in milliseconds.
    pub delay_min_ms: u64,
    /// Maximum delay in milliseconds.
    pub delay_max_ms: u64,
    /// Probability of simulating a timeout (0.0 - 1.0).
    pub timeout_rate: f64,
    /// Timeout value in milliseconds (used in Timeout error).
    pub timeout_ms: u64,
}

impl Default for FailInjectionConfig {
    fn default() -> Self {
        Self {
            failure_rate: 0.0,
            delay_min_ms: 0,
            delay_max_ms: 0,
            timeout_rate: 0.0,
            timeout_ms: 5000,
        }
    }
}

impl FailInjectionConfig {
    /// Creates a configuration from environment variables.
    ///
    /// # Environment Variables
    ///
    /// - `{prefix}_FAILURE_RATE`: Failure rate (0.0 - 1.0)
    /// - `{prefix}_DELAY_MIN_MS`: Minimum delay in milliseconds
    /// - `{prefix}_DELAY_MAX_MS`: Maximum delay in milliseconds
    /// - `{prefix}_TIMEOUT_RATE`: Timeout rate (0.0 - 1.0)
    /// - `EXTERNAL_TIMEOUT_MS`: Timeout value (shared across sources)
    ///
    /// # Errors
    ///
    /// Returns an error if any environment variable contains an invalid value,
    /// or if the configuration fails validation.
    /// Missing variables use default values.
    pub fn from_env(prefix: &str) -> Result<Self, ConfigError> {
        let config = Self {
            failure_rate: parse_env_f64(&format!("{prefix}_FAILURE_RATE"), 0.0)?,
            delay_min_ms: parse_env_u64(&format!("{prefix}_DELAY_MIN_MS"), 0)?,
            delay_max_ms: parse_env_u64(&format!("{prefix}_DELAY_MAX_MS"), 0)?,
            timeout_rate: parse_env_f64(&format!("{prefix}_TIMEOUT_RATE"), 0.0)?,
            timeout_ms: parse_env_u64("EXTERNAL_TIMEOUT_MS", 5000)?,
        };
        config.validate()?;
        Ok(config)
    }

    /// Creates a deterministic configuration for testing/benchmarking.
    ///
    /// This method does not read from environment variables, making it
    /// suitable for reproducible tests.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration fails validation.
    pub fn deterministic(
        failure_rate: f64,
        delay_ms: u64,
        timeout_rate: f64,
        timeout_ms: u64,
    ) -> Result<Self, ConfigError> {
        let config = Self {
            failure_rate,
            delay_min_ms: delay_ms,
            delay_max_ms: delay_ms,
            timeout_rate,
            timeout_ms,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validates the configuration values.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `failure_rate` is not in range `0.0..=1.0`
    /// - `timeout_rate` is not in range `0.0..=1.0`
    /// - `delay_min_ms > delay_max_ms`
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(0.0..=1.0).contains(&self.failure_rate) {
            return Err(ConfigError::InvalidFailureRate(self.failure_rate));
        }
        if !(0.0..=1.0).contains(&self.timeout_rate) {
            return Err(ConfigError::InvalidTimeoutRate(self.timeout_rate));
        }
        if self.delay_min_ms > self.delay_max_ms {
            return Err(ConfigError::InvalidDelayRange {
                min: self.delay_min_ms,
                max: self.delay_max_ms,
            });
        }
        Ok(())
    }

    /// Returns whether fail injection is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.failure_rate > 0.0 || self.delay_max_ms > 0 || self.timeout_rate > 0.0
    }
}

// =============================================================================
// Environment Variable Parsing
// =============================================================================

/// Error type for environment variable parsing.
#[derive(Debug, Error)]
pub enum EnvParseError {
    /// Invalid f64 value.
    #[error("Invalid f64 value for {name}: {message} (got '{value}')")]
    InvalidF64 {
        /// Variable name.
        name: String,
        /// Error message.
        message: String,
        /// Actual value.
        value: String,
    },

    /// Invalid u64 value.
    #[error("Invalid u64 value for {name}: {message} (got '{value}')")]
    InvalidU64 {
        /// Variable name.
        name: String,
        /// Error message.
        message: String,
        /// Actual value.
        value: String,
    },
}

/// Parses an f64 from an environment variable.
///
/// Returns the default value if the variable is not set.
/// Returns an error if the variable is set but contains an invalid value.
fn parse_env_f64(name: &str, default: f64) -> Result<f64, EnvParseError> {
    match std::env::var(name) {
        Ok(value) => {
            value
                .parse()
                .map_err(|e: std::num::ParseFloatError| EnvParseError::InvalidF64 {
                    name: name.to_string(),
                    message: e.to_string(),
                    value,
                })
        }
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(EnvParseError::InvalidF64 {
            name: name.to_string(),
            message: error.to_string(),
            value: String::new(),
        }),
    }
}

/// Parses a u64 from an environment variable.
///
/// Returns the default value if the variable is not set.
/// Returns an error if the variable is set but contains an invalid value.
fn parse_env_u64(name: &str, default: u64) -> Result<u64, EnvParseError> {
    match std::env::var(name) {
        Ok(value) => {
            value
                .parse()
                .map_err(|e: std::num::ParseIntError| EnvParseError::InvalidU64 {
                    name: name.to_string(),
                    message: e.to_string(),
                    value,
                })
        }
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(EnvParseError::InvalidU64 {
            name: name.to_string(),
            message: error.to_string(),
            value: String::new(),
        }),
    }
}

// =============================================================================
// Configuration Errors
// =============================================================================

/// Configuration validation errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Invalid RNG seed value.
    #[error("Invalid RNG_SEED: {message} (got '{value}')")]
    InvalidRngSeed {
        /// Error message.
        message: String,
        /// Actual value.
        value: String,
    },

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),

    /// Environment parsing error.
    #[error("Environment parsing error: {0}")]
    EnvParseError(#[from] EnvParseError),

    /// Failure rate out of range.
    #[error("Invalid failure rate: must be 0.0-1.0, got {0}")]
    InvalidFailureRate(f64),

    /// Timeout rate out of range.
    #[error("Invalid timeout rate: must be 0.0-1.0, got {0}")]
    InvalidTimeoutRate(f64),

    /// Invalid delay range.
    #[error("Invalid delay range: min ({min}) > max ({max})")]
    InvalidDelayRange {
        /// Minimum delay.
        min: u64,
        /// Maximum delay.
        max: u64,
    },
}

// =============================================================================
// RNG Provider
// =============================================================================

/// RNG provider for deterministic/random behavior.
///
/// This provider supports two modes:
/// - **Random mode**: Uses thread-local RNG for non-deterministic behavior
/// - **Seeded mode**: Uses a fixed seed for deterministic, reproducible behavior
///
/// # Concurrency Considerations
///
/// When using shared RNG in concurrent scenarios, the order of random number
/// consumption depends on thread scheduling. To ensure deterministic behavior
/// even in concurrent execution, use `for_operation` to create operation-scoped
/// RNG instances with derived seeds.
///
/// # Child Seed Derivation
///
/// Child seeds are derived using `SipHash-2-4` with a fixed key:
/// ```text
/// child_seed = SipHash24(key=[0u8;16], data=concat(parent_seed.to_le_bytes(), scope_key.as_bytes()))
/// ```
///
/// This ensures:
/// - Same parent seed + same scope = same child seed
/// - Different scopes = different child seeds
/// - Deterministic across runs and platforms
pub struct RngProvider {
    /// Parent seed for child RNG generation. None = random mode.
    parent_seed: Option<u64>,
}

impl RngProvider {
    /// Creates a new RNG provider in random mode.
    ///
    /// Each call to `for_operation` will create a new thread-local RNG.
    #[must_use]
    pub const fn new_random() -> Self {
        Self { parent_seed: None }
    }

    /// Creates a new RNG provider in seeded mode.
    ///
    /// Each call to `for_operation` will create an RNG with a deterministically
    /// derived seed based on the parent seed and scope key.
    #[must_use]
    pub const fn new_seeded(seed: u64) -> Self {
        Self {
            parent_seed: Some(seed),
        }
    }

    /// Creates a seeded RNG provider from environment variable.
    ///
    /// Reads `RNG_SEED` environment variable:
    /// - If set: Uses the seed value for deterministic mode
    /// - If not set: Uses random mode
    ///
    /// # Errors
    ///
    /// Returns an error if `RNG_SEED` is set but contains an invalid value.
    pub fn from_env() -> Result<Self, ConfigError> {
        match std::env::var("RNG_SEED") {
            Ok(seed_str) => {
                let seed: u64 = seed_str.parse().map_err(|e: std::num::ParseIntError| {
                    ConfigError::InvalidRngSeed {
                        message: e.to_string(),
                        value: seed_str,
                    }
                })?;
                tracing::info!(seed = seed, "Using deterministic RNG");
                Ok(Self::new_seeded(seed))
            }
            Err(std::env::VarError::NotPresent) => Ok(Self::new_random()),
            Err(error) => Err(ConfigError::EnvVarError(error)),
        }
    }

    /// Creates a scoped RNG for a specific operation.
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task ID (request scope)
    /// * `source_name` - Source name (`"secondary"`, `"external"`, etc.)
    /// * `operation` - Operation name (`"fetch_task_data"`, etc.)
    ///
    /// # Scope Isolation
    ///
    /// Different combinations of (`task_id`, `source_name`, `operation`) produce
    /// different random sequences, ensuring that:
    /// - Multiple sources in the same request get different sequences
    /// - Retries can use different sequences by varying the operation name
    ///
    /// # Same Scope = Same Sequence
    ///
    /// Calling with the same scope key will produce the same sequence.
    /// This is intentional for deterministic behavior.
    #[must_use]
    #[allow(clippy::option_if_let_else)] // match is clearer for this case
    pub fn for_operation(&self, task_id: &str, source_name: &str, operation: &str) -> ScopedRng {
        match self.parent_seed {
            Some(parent_seed) => {
                // Derive child seed using SipHash-2-4 with fixed key
                let mut hasher = SipHasher24::new_with_key(&[0u8; 16]);
                hasher.write(&parent_seed.to_le_bytes());
                hasher.write(task_id.as_bytes());
                hasher.write(b":");
                hasher.write(source_name.as_bytes());
                hasher.write(b":");
                hasher.write(operation.as_bytes());
                let child_seed = hasher.finish();

                ScopedRng(StdRng::seed_from_u64(child_seed))
            }
            None => {
                // For random mode, use entropy source to generate a seed
                // This creates a Send-safe StdRng instead of ThreadRng
                ScopedRng(StdRng::from_os_rng())
            }
        }
    }
}

// =============================================================================
// Scoped RNG
// =============================================================================

/// Operation-scoped RNG.
///
/// This wraps a `StdRng` which is `Send + Sync`, making it safe to use
/// across await points in async code.
///
/// The RNG is created by `RngProvider::for_operation` and should only
/// be used within `AsyncIO` closures (I/O boundary).
pub struct ScopedRng(StdRng);

impl ScopedRng {
    /// Generates a random f64 in range [0.0, 1.0).
    ///
    /// This is an I/O operation (side effect: RNG state mutation).
    pub fn random_f64(&mut self) -> f64 {
        self.0.random()
    }

    /// Generates a random u64 in range [min, max].
    ///
    /// This is an I/O operation (side effect: RNG state mutation).
    pub fn random_range(&mut self, min: u64, max: u64) -> u64 {
        self.0.random_range(min..=max)
    }
}

// =============================================================================
// Post-Injection Application
// =============================================================================

/// Applies fail injection after successful I/O.
///
/// This function should only be called after real I/O succeeds.
/// It applies the following in order:
/// 1. Delay injection (if configured)
/// 2. Failure injection (based on `failure_rate`)
/// 3. Timeout simulation (based on `timeout_rate`)
///
/// # I/O Boundary
///
/// This function contains I/O operations (RNG, sleep) and should only
/// be called within `AsyncIO` closures.
///
/// # Errors
///
/// Returns an error if fail injection determines a failure should occur.
pub async fn apply_post_injection(
    config: &FailInjectionConfig,
    rng: &mut ScopedRng,
) -> Result<(), ExternalError> {
    // 1. Apply delay injection (I/O: RNG + sleep)
    let delay_ms = compute_delay(config, rng);
    if delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // 2. Apply failure injection (I/O: RNG)
    if rng.random_f64() < config.failure_rate {
        return Err(ExternalError::InjectedFailure(
            "Random failure injection".to_string(),
        ));
    }

    // 3. Apply timeout simulation (I/O: RNG)
    if rng.random_f64() < config.timeout_rate {
        return Err(ExternalError::Timeout(config.timeout_ms));
    }

    Ok(())
}

/// Computes delay value based on configuration.
///
/// This function contains I/O (RNG) and should be called within I/O boundary.
fn compute_delay(config: &FailInjectionConfig, rng: &mut ScopedRng) -> u64 {
    if config.delay_max_ms == 0 {
        0
    } else if config.delay_min_ms == config.delay_max_ms {
        config.delay_min_ms
    } else {
        rng.random_range(config.delay_min_ms, config.delay_max_ms)
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
    // FailInjectionConfig Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_fail_injection_config_default() {
        let config = FailInjectionConfig::default();
        assert!((config.failure_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(config.delay_min_ms, 0);
        assert_eq!(config.delay_max_ms, 0);
        assert!((config.timeout_rate - 0.0).abs() < f64::EPSILON);
        assert_eq!(config.timeout_ms, 5000);
        assert!(!config.is_enabled());
    }

    #[rstest]
    fn test_fail_injection_config_deterministic() {
        let config = FailInjectionConfig::deterministic(0.5, 100, 0.1, 3000).unwrap();
        assert!((config.failure_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.delay_min_ms, 100);
        assert_eq!(config.delay_max_ms, 100);
        assert!((config.timeout_rate - 0.1).abs() < f64::EPSILON);
        assert_eq!(config.timeout_ms, 3000);
        assert!(config.is_enabled());
    }

    #[rstest]
    fn test_fail_injection_config_validate_valid() {
        let config = FailInjectionConfig::deterministic(0.5, 100, 0.1, 3000).unwrap();
        assert!(config.validate().is_ok());
    }

    #[rstest]
    fn test_fail_injection_config_validate_invalid_failure_rate_too_high() {
        let config = FailInjectionConfig {
            failure_rate: 1.5,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::InvalidFailureRate(_))));
    }

    #[rstest]
    fn test_fail_injection_config_validate_invalid_failure_rate_negative() {
        let config = FailInjectionConfig {
            failure_rate: -0.1,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::InvalidFailureRate(_))));
    }

    #[rstest]
    fn test_fail_injection_config_validate_invalid_timeout_rate() {
        let config = FailInjectionConfig {
            timeout_rate: 2.0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::InvalidTimeoutRate(_))));
    }

    #[rstest]
    fn test_fail_injection_config_validate_invalid_delay_range() {
        let config = FailInjectionConfig {
            delay_min_ms: 200,
            delay_max_ms: 100,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::InvalidDelayRange { .. })));
    }

    #[rstest]
    fn test_fail_injection_config_is_enabled_with_failure_rate() {
        let config = FailInjectionConfig {
            failure_rate: 0.1,
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    #[rstest]
    fn test_fail_injection_config_is_enabled_with_delay() {
        let config = FailInjectionConfig {
            delay_max_ms: 100,
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    #[rstest]
    fn test_fail_injection_config_is_enabled_with_timeout_rate() {
        let config = FailInjectionConfig {
            timeout_rate: 0.1,
            ..Default::default()
        };
        assert!(config.is_enabled());
    }

    // -------------------------------------------------------------------------
    // RngProvider Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_rng_provider_new_random() {
        let provider = RngProvider::new_random();
        assert!(provider.parent_seed.is_none());
    }

    #[rstest]
    fn test_rng_provider_new_seeded() {
        let provider = RngProvider::new_seeded(12345);
        assert_eq!(provider.parent_seed, Some(12345));
    }

    #[rstest]
    fn test_rng_provider_deterministic_same_scope() {
        // Same seed + same scope = same sequence
        let provider = RngProvider::new_seeded(12345);

        let mut rng1 = provider.for_operation("task-1", "secondary", "fetch");
        let vals1: Vec<f64> = (0..10).map(|_| rng1.random_f64()).collect();

        let mut rng2 = provider.for_operation("task-1", "secondary", "fetch");
        let vals2: Vec<f64> = (0..10).map(|_| rng2.random_f64()).collect();

        assert_eq!(vals1, vals2, "Same scope should produce same sequence");
    }

    #[rstest]
    fn test_rng_provider_scope_isolation_by_source() {
        let provider = RngProvider::new_seeded(12345);

        let mut rng_secondary = provider.for_operation("task-1", "secondary", "fetch");
        let mut rng_external = provider.for_operation("task-1", "external", "fetch");

        let vals_secondary: Vec<f64> = (0..10).map(|_| rng_secondary.random_f64()).collect();
        let vals_external: Vec<f64> = (0..10).map(|_| rng_external.random_f64()).collect();

        assert_ne!(
            vals_secondary, vals_external,
            "Different sources should produce different sequences"
        );
    }

    #[rstest]
    fn test_rng_provider_scope_isolation_by_task_id() {
        let provider = RngProvider::new_seeded(12345);

        let mut rng_task1 = provider.for_operation("task-1", "secondary", "fetch");
        let mut rng_task2 = provider.for_operation("task-2", "secondary", "fetch");

        let vals_task1: Vec<f64> = (0..10).map(|_| rng_task1.random_f64()).collect();
        let vals_task2: Vec<f64> = (0..10).map(|_| rng_task2.random_f64()).collect();

        assert_ne!(
            vals_task1, vals_task2,
            "Different task IDs should produce different sequences"
        );
    }

    #[rstest]
    fn test_rng_provider_scope_isolation_by_operation() {
        let provider = RngProvider::new_seeded(12345);

        let mut rng_fetch = provider.for_operation("task-1", "secondary", "fetch");
        let mut rng_retry = provider.for_operation("task-1", "secondary", "fetch:retry_1");

        let vals_fetch: Vec<f64> = (0..10).map(|_| rng_fetch.random_f64()).collect();
        let vals_retry: Vec<f64> = (0..10).map(|_| rng_retry.random_f64()).collect();

        assert_ne!(
            vals_fetch, vals_retry,
            "Different operations should produce different sequences"
        );
    }

    // -------------------------------------------------------------------------
    // apply_post_injection Tests
    // -------------------------------------------------------------------------

    #[rstest]
    #[tokio::test]
    async fn test_apply_post_injection_never_fails() {
        let config = FailInjectionConfig::deterministic(0.0, 0, 0.0, 5000).unwrap();
        let provider = RngProvider::new_seeded(0);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let result = apply_post_injection(&config, &mut rng).await;
        assert!(result.is_ok());
    }

    #[rstest]
    #[tokio::test]
    async fn test_apply_post_injection_always_fails() {
        let config = FailInjectionConfig::deterministic(1.0, 0, 0.0, 5000).unwrap();
        let provider = RngProvider::new_seeded(0);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let result = apply_post_injection(&config, &mut rng).await;
        assert!(matches!(result, Err(ExternalError::InjectedFailure(_))));
    }

    #[rstest]
    #[tokio::test]
    async fn test_apply_post_injection_always_timeout() {
        // failure_rate=0 so failure check passes, then timeout_rate=1.0
        let config = FailInjectionConfig::deterministic(0.0, 0, 1.0, 3000).unwrap();
        let provider = RngProvider::new_seeded(0);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let result = apply_post_injection(&config, &mut rng).await;
        assert!(matches!(result, Err(ExternalError::Timeout(3000))));
    }

    #[rstest]
    #[tokio::test]
    async fn test_apply_post_injection_deterministic() {
        // Same config + same seed + same scope = same result
        let config = FailInjectionConfig::deterministic(0.5, 0, 0.0, 5000).unwrap();

        let provider1 = RngProvider::new_seeded(12345);
        let mut rng1 = provider1.for_operation("task-1", "secondary", "fetch");
        let result1 = apply_post_injection(&config, &mut rng1).await;

        let provider2 = RngProvider::new_seeded(12345);
        let mut rng2 = provider2.for_operation("task-1", "secondary", "fetch");
        let result2 = apply_post_injection(&config, &mut rng2).await;

        match (&result1, &result2) {
            (Ok(()), Ok(())) => {}
            (Err(ExternalError::InjectedFailure(m1)), Err(ExternalError::InjectedFailure(m2))) => {
                assert_eq!(m1, m2);
            }
            (Err(ExternalError::Timeout(t1)), Err(ExternalError::Timeout(t2))) => {
                assert_eq!(t1, t2);
            }
            _ => panic!("Results differ: {result1:?} vs {result2:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // compute_delay Tests
    // -------------------------------------------------------------------------

    #[rstest]
    fn test_compute_delay_zero_max() {
        let config = FailInjectionConfig {
            delay_max_ms: 0,
            ..Default::default()
        };
        let provider = RngProvider::new_seeded(12345);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let delay = compute_delay(&config, &mut rng);
        assert_eq!(delay, 0);
    }

    #[rstest]
    fn test_compute_delay_fixed_value() {
        let config = FailInjectionConfig {
            delay_min_ms: 100,
            delay_max_ms: 100,
            ..Default::default()
        };
        let provider = RngProvider::new_seeded(12345);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let delay = compute_delay(&config, &mut rng);
        assert_eq!(delay, 100);
    }

    #[rstest]
    fn test_compute_delay_range() {
        let config = FailInjectionConfig {
            delay_min_ms: 50,
            delay_max_ms: 150,
            ..Default::default()
        };
        let provider = RngProvider::new_seeded(12345);
        let mut rng = provider.for_operation("task-1", "secondary", "fetch");

        let delay = compute_delay(&config, &mut rng);
        assert!((50..=150).contains(&delay));
    }
}
