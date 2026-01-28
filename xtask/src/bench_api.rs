//! API benchmark runner
//!
//! This module provides the `bench-api` subcommand for running API benchmarks
//! with scenario configuration.

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Arguments for the bench-api subcommand
#[derive(Args, Debug)]
pub struct BenchApiArgs {
    /// Scenario YAML file path (required)
    #[arg(long, short = 's')]
    pub scenario: PathBuf,

    /// Enable profiling (perf + flamegraph)
    #[arg(long)]
    pub profile: bool,

    /// Quick mode (5s duration)
    #[arg(long)]
    pub quick: bool,

    /// Override storage mode (in_memory|postgres)
    #[arg(long)]
    pub storage: Option<String>,

    /// Override cache mode (in_memory|redis|none)
    #[arg(long)]
    pub cache: Option<String>,

    /// Override cache strategy (read-through|write-through|write-behind)
    #[arg(long)]
    pub cache_strategy: Option<String>,

    /// Override hit rate (0-100)
    #[arg(long)]
    pub hit_rate: Option<u8>,

    /// Override fail injection rate (0.0-1.0)
    #[arg(long)]
    pub fail_rate: Option<f64>,

    /// Override data scale (small|medium|large)
    #[arg(long)]
    pub data_scale: Option<String>,

    /// Override payload variant (minimal|standard|complex|heavy)
    #[arg(long)]
    pub payload: Option<String>,

    /// Random seed for reproducible data
    #[arg(long)]
    pub seed: Option<u64>,

    /// Skip data setup
    #[arg(long)]
    pub skip_setup: bool,

    /// Skip API health check
    #[arg(long)]
    pub skip_health_check: bool,

    /// Skip API startup (assume already running)
    #[arg(long)]
    pub skip_api_start: bool,

    /// Don't stop API after benchmark
    #[arg(long)]
    pub keep_api_running: bool,

    /// Load environment from file (default: .env.sample)
    #[arg(long, default_value = "benches/api/benchmarks/.env.sample")]
    pub env_file: PathBuf,

    /// Dry-run mode: show commands without executing
    #[arg(long)]
    pub dry_run: bool,
}

/// Scenario configuration from YAML
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScenarioConfig {
    name: Option<String>,
    storage_mode: Option<String>,
    cache_mode: Option<String>,
    cache_strategy: Option<String>,
    data_scale: Option<String>,
    payload_variant: Option<String>,
    load_pattern: Option<String>,
    duration_seconds: Option<u32>,
    connections: Option<u32>,
    threads: Option<u32>,
    warmup_seconds: Option<u32>,
    target_rps: Option<u32>,
    pool_sizes: Option<PoolSizes>,
    concurrency: Option<ConcurrencyConfig>,
    worker_config: Option<WorkerConfig>,
    thresholds: Option<HashMap<String, serde_yaml::Value>>,
    metadata: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Deserialize)]
struct PoolSizes {
    database_pool_size: Option<u32>,
    redis_pool_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ConcurrencyConfig {
    worker_threads: Option<u32>,
    database_pool_size: Option<u32>,
    redis_pool_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct WorkerConfig {
    worker_threads: Option<u32>,
}

/// Environment variables to set for benchmark execution
struct BenchEnv {
    storage_mode: String,
    cache_mode: String,
    cache_strategy: String,
    hit_rate: u8,
    fail_rate: f64,
    worker_threads: Option<u32>,
    database_pool_size: Option<u32>,
    redis_pool_size: Option<u32>,
    data_scale: String,
    payload_variant: String,
    seed: Option<u64>,
    api_url: String,
    profile: bool,
}

impl BenchEnv {
    fn from_args_and_scenario(
        args: &BenchApiArgs,
        scenario: &ScenarioConfig,
        root: &Path,
    ) -> Result<Self> {
        // Resolve env_file relative to project root if not absolute
        let env_file_path = if args.env_file.is_absolute() {
            args.env_file.clone()
        } else {
            root.join(&args.env_file)
        };

        // Load .env file if exists
        let env_vars = load_env_file(&env_file_path).unwrap_or_default();

        // Priority: CLI > Environment > .env file > Scenario YAML > Default

        let storage_mode = args
            .storage
            .clone()
            .or_else(|| env::var("STORAGE_MODE").ok())
            .or_else(|| env_vars.get("STORAGE_MODE").cloned())
            .or_else(|| scenario.storage_mode.clone())
            .unwrap_or_else(|| "in_memory".to_string());

        let cache_mode = args
            .cache
            .clone()
            .or_else(|| env::var("CACHE_MODE").ok())
            .or_else(|| env_vars.get("CACHE_MODE").cloned())
            .or_else(|| scenario.cache_mode.clone())
            .unwrap_or_else(|| "redis".to_string());

        let cache_strategy = args
            .cache_strategy
            .clone()
            .or_else(|| env::var("CACHE_STRATEGY").ok())
            .or_else(|| env_vars.get("CACHE_STRATEGY").cloned())
            .or_else(|| scenario.cache_strategy.clone())
            .unwrap_or_else(|| "read-through".to_string());

        let hit_rate = args.hit_rate.or_else(|| {
            env::var("HIT_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .or_else(|| env_vars.get("HIT_RATE").and_then(|v| v.parse().ok()))
                .or_else(|| {
                    scenario
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("hit_rate"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u8)
                })
        });
        let hit_rate = hit_rate.unwrap_or(50);

        let fail_rate = args.fail_rate.or_else(|| {
            env::var("FAIL_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .or_else(|| env_vars.get("FAIL_RATE").and_then(|v| v.parse().ok()))
                .or_else(|| {
                    scenario
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("fail_injection"))
                        .and_then(|v| v.as_f64())
                })
        });
        let fail_rate = fail_rate.unwrap_or(0.0);

        // Worker threads: from environment, concurrency.worker_threads, or worker_config.worker_threads
        // None means use library default (don't set environment variable)
        let worker_threads = env::var("WORKER_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| env_vars.get("WORKER_THREADS").and_then(|v| v.parse().ok()))
            .or_else(|| scenario.concurrency.as_ref().and_then(|c| c.worker_threads))
            .or_else(|| {
                scenario
                    .worker_config
                    .as_ref()
                    .and_then(|w| w.worker_threads)
            });

        // Pool sizes: from environment, concurrency, or pool_sizes
        // None means use library default (don't set environment variable)
        let database_pool_size = env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| {
                env_vars
                    .get("DATABASE_POOL_SIZE")
                    .and_then(|v| v.parse().ok())
            })
            .or_else(|| {
                scenario
                    .concurrency
                    .as_ref()
                    .and_then(|c| c.database_pool_size)
            })
            .or_else(|| {
                scenario
                    .pool_sizes
                    .as_ref()
                    .and_then(|p| p.database_pool_size)
            });

        let redis_pool_size = env::var("REDIS_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| env_vars.get("REDIS_POOL_SIZE").and_then(|v| v.parse().ok()))
            .or_else(|| {
                scenario
                    .concurrency
                    .as_ref()
                    .and_then(|c| c.redis_pool_size)
            })
            .or_else(|| scenario.pool_sizes.as_ref().and_then(|p| p.redis_pool_size));

        let data_scale = args
            .data_scale
            .clone()
            .or_else(|| env::var("DATA_SCALE").ok())
            .or_else(|| env_vars.get("DATA_SCALE").cloned())
            .or_else(|| scenario.data_scale.clone())
            .unwrap_or_else(|| "small".to_string());

        let payload_variant = args
            .payload
            .clone()
            .or_else(|| env::var("PAYLOAD_VARIANT").ok())
            .or_else(|| env_vars.get("PAYLOAD_VARIANT").cloned())
            .or_else(|| scenario.payload_variant.clone())
            .unwrap_or_else(|| "standard".to_string());

        let seed = args.seed.or_else(|| {
            env::var("SEED")
                .ok()
                .and_then(|v| v.parse().ok())
                .or_else(|| env_vars.get("SEED").and_then(|v| v.parse().ok()))
        });

        let api_url = env::var("API_URL")
            .ok()
            .or_else(|| env_vars.get("API_URL").cloned())
            .unwrap_or_else(|| "http://localhost:3002".to_string());

        Ok(Self {
            storage_mode,
            cache_mode,
            cache_strategy,
            hit_rate,
            fail_rate,
            worker_threads,
            database_pool_size,
            redis_pool_size,
            data_scale,
            payload_variant,
            seed,
            api_url,
            profile: args.profile,
        })
    }

    /// # Safety
    /// Uses `env::set_var` (unsafe in Rust 2024). Must be called from single-threaded
    /// context before spawning child processes. xtask runner is single-threaded.
    fn set_env_vars(&self) {
        // SAFETY: xtask is single-threaded, environment is inherited by child processes
        unsafe {
            env::set_var("STORAGE_MODE", &self.storage_mode);
            env::set_var("CACHE_MODE", &self.cache_mode);
            env::set_var("CACHE_STRATEGY", &self.cache_strategy);
            env::set_var("HIT_RATE", self.hit_rate.to_string());
            env::set_var("FAIL_RATE", self.fail_rate.to_string());
            env::set_var("DATA_SCALE", &self.data_scale);
            env::set_var("PAYLOAD_VARIANT", &self.payload_variant);
            env::set_var("API_URL", &self.api_url);

            if let Some(threads) = self.worker_threads {
                env::set_var("WORKER_THREADS", threads.to_string());
            }
            if let Some(size) = self.database_pool_size {
                env::set_var("DATABASE_POOL_SIZE", size.to_string());
            }
            if let Some(size) = self.redis_pool_size {
                env::set_var("REDIS_POOL_SIZE", size.to_string());
            }

            if let Some(seed) = self.seed {
                env::set_var("SEED", seed.to_string());
            }

            if self.profile {
                env::set_var("PROFILE", "true");
            }
        }
    }

    /// Generate a string of environment variables for Docker compose command display
    fn to_docker_env_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(threads) = self.worker_threads {
            parts.push(format!("WORKER_THREADS={}", threads));
        }
        if let Some(size) = self.database_pool_size {
            parts.push(format!("DATABASE_POOL_SIZE={}", size));
        }
        if let Some(size) = self.redis_pool_size {
            parts.push(format!("REDIS_POOL_SIZE={}", size));
        }

        parts.push(format!("STORAGE_MODE={}", self.storage_mode));
        parts.push(format!("CACHE_MODE={}", self.cache_mode));
        parts.push(format!("CACHE_STRATEGY={}", self.cache_strategy));
        parts.push("ENABLE_DEBUG_ENDPOINTS=true".to_string());

        parts.join(" ")
    }
}

fn load_env_file(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path).context("Failed to read env file")?;
    let mut vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            vars.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(vars)
}

fn project_root() -> Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    // xtask is in project_root/xtask, so go up one level
    let root = if manifest_dir.ends_with("xtask") {
        manifest_dir.parent().unwrap().to_path_buf()
    } else {
        manifest_dir
    };

    Ok(root)
}

fn check_api_health(api_url: &str, max_retries: u32) -> Result<bool> {
    for attempt in 1..=max_retries {
        let health_url = format!("{}/health", api_url);
        let result = Command::new("curl")
            .args(["-sf", &health_url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if result.map(|s| s.success()).unwrap_or(false) {
            return Ok(true);
        }

        if attempt < max_retries {
            eprintln!(
                "  Health check attempt {}/{} failed, retrying in 2s...",
                attempt, max_retries
            );
            thread::sleep(Duration::from_secs(2));
        }
    }

    Ok(false)
}

/// Ensures API containers are stopped on drop (RAII cleanup guard)
///
/// # Behavior
///
/// This guard provides automatic cleanup of API containers when dropped.
/// The `should_stop` flag controls whether cleanup occurs:
///
/// - **Normal exit path**: If `keep_api_running` is set, call `keep_running()` to prevent cleanup
/// - **Abnormal exit path**: The guard will always stop the API on drop if `should_stop` is true,
///   ensuring containers don't leak even when errors occur. This is intentional behavior.
///
/// This means `keep_api_running` only affects successful completion - if an error occurs,
/// the API will be stopped regardless of the flag value.
struct ApiGuard<'a> {
    root: &'a Path,
    should_stop: bool,
}

impl<'a> ApiGuard<'a> {
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            should_stop: false,
        }
    }

    fn mark_started(&mut self) {
        self.should_stop = true;
    }
    fn keep_running(&mut self) {
        self.should_stop = false;
    }
    fn will_stop(&self) -> bool {
        self.should_stop
    }
}

impl Drop for ApiGuard<'_> {
    fn drop(&mut self) {
        if self.should_stop {
            eprintln!("Stopping API (cleanup from guard)...");
            if let Err(error) = stop_api(self.root) {
                eprintln!("Warning: Failed to stop API in cleanup: {}", error);
            }
        }
    }
}

fn start_api(root: &Path, environment: &BenchEnv) -> Result<()> {
    let compose_file = root.join("benches/api/docker/compose.ci.yaml");

    if !compose_file.exists() {
        bail!("Docker compose file not found: {}", compose_file.display());
    }

    // If API is already running, stop it first to ensure new environment is applied (ENV-REQ-010)
    if check_api_health(&environment.api_url, 1)? {
        eprintln!("API is already running, restarting to apply new environment...");
        stop_api(root)?;
    }

    eprintln!("Starting API with docker compose...");

    let mut command = Command::new("docker");
    command
        .args([
            "compose",
            "-f",
            &compose_file.to_string_lossy(),
            "up",
            "-d",
            "--build",
            "--wait",
        ])
        .current_dir(root);

    if let Some(threads) = environment.worker_threads {
        command.env("WORKER_THREADS", threads.to_string());
    }
    if let Some(size) = environment.database_pool_size {
        command.env("DATABASE_POOL_SIZE", size.to_string());
    }
    if let Some(size) = environment.redis_pool_size {
        command.env("REDIS_POOL_SIZE", size.to_string());
    }

    command
        .env("STORAGE_MODE", &environment.storage_mode)
        .env("CACHE_MODE", &environment.cache_mode)
        .env("CACHE_STRATEGY", &environment.cache_strategy)
        .env("ENABLE_DEBUG_ENDPOINTS", "true"); // Enable /debug/config for benchmarks

    let status = command.status().context("Failed to start docker compose")?;

    if !status.success() {
        // Clean up any partially started containers before returning error
        let _ = stop_api(root);
        bail!("Failed to start API via docker compose");
    }

    Ok(())
}

fn stop_api(root: &Path) -> Result<()> {
    let compose_file = root.join("benches/api/docker/compose.ci.yaml");

    if !compose_file.exists() {
        return Ok(());
    }

    eprintln!("Stopping API...");

    let status = Command::new("docker")
        .args([
            "compose",
            "-f",
            &compose_file.to_string_lossy(),
            "down",
            "-v",
        ])
        .current_dir(root)
        .status()
        .context("Failed to execute docker compose down")?;

    if !status.success() {
        bail!(
            "Failed to stop API via docker compose (exit code: {:?})",
            status.code()
        );
    }

    Ok(())
}

fn setup_test_data(root: &Path, bench_env: &BenchEnv) -> Result<()> {
    let setup_script = root.join("benches/api/benchmarks/setup_test_data.sh");

    if !setup_script.exists() {
        bail!("Setup script not found: {}", setup_script.display());
    }

    eprintln!("Setting up test data...");

    let mut cmd = Command::new(&setup_script);
    cmd.current_dir(root)
        .arg("--scale")
        .arg(&bench_env.data_scale)
        .arg("--payload")
        .arg(&bench_env.payload_variant);

    if let Some(seed) = bench_env.seed {
        cmd.arg("--seed").arg(seed.to_string());
    }

    // Output metadata to temporary file for later integration
    let meta_output = root.join("benches/api/benchmarks/setup_meta.json");
    cmd.arg("--meta-output").arg(&meta_output);

    let status = cmd.status().context("Failed to run setup script")?;

    if !status.success() {
        bail!("Test data setup failed");
    }

    Ok(())
}

fn run_benchmark(root: &Path, scenario: &Path, quick: bool) -> Result<PathBuf> {
    let benchmark_script = root.join("benches/api/benchmarks/run_benchmark.sh");

    if !benchmark_script.exists() {
        bail!("Benchmark script not found: {}", benchmark_script.display());
    }

    eprintln!("Running benchmark...");

    let mut cmd = Command::new(&benchmark_script);
    cmd.current_dir(root).arg("--scenario").arg(scenario);

    if quick {
        cmd.arg("--quick");
    }

    let output = cmd.output().context("Failed to run benchmark script")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Benchmark failed: {}", stderr);
    }

    // Parse output to find results directory
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{}", stdout);

    // Extract results directory from output
    // Expected format: "Results saved to: results/<timestamp>/<scenario>/"
    for line in stdout.lines() {
        let contains_results = line.contains("Results saved to:") || line.contains("results/");
        if contains_results && let Some(path_str) = line.split_whitespace().last() {
            let results_path = root.join(path_str);
            if results_path.exists() {
                return Ok(results_path);
            }
        }
    }

    // Fallback: find the most recent results directory
    let results_base = root.join("benches/api/benchmarks/results");
    if results_base.exists()
        && let Ok(entries) = fs::read_dir(&results_base)
    {
        let mut dirs: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        dirs.sort_by_key(|e| e.path());

        if let Some(latest) = dirs.last() {
            return Ok(latest.path());
        }
    }

    bail!("Could not determine results directory");
}

fn print_dry_run_output(
    bench_env: &BenchEnv,
    scenario_path: &Path,
    root: &Path,
    args: &BenchApiArgs,
) {
    eprintln!("=== DRY RUN MODE ===");
    eprintln!();

    // Environment variables to be applied
    eprintln!("Environment variables to be applied:");
    if let Some(threads) = bench_env.worker_threads {
        eprintln!("  WORKER_THREADS={}", threads);
    }
    if let Some(size) = bench_env.database_pool_size {
        eprintln!("  DATABASE_POOL_SIZE={}", size);
    }
    if let Some(size) = bench_env.redis_pool_size {
        eprintln!("  REDIS_POOL_SIZE={}", size);
    }
    eprintln!("  STORAGE_MODE={}", bench_env.storage_mode);
    eprintln!("  CACHE_MODE={}", bench_env.cache_mode);
    eprintln!("  CACHE_STRATEGY={}", bench_env.cache_strategy);
    eprintln!("  HIT_RATE={}", bench_env.hit_rate);
    eprintln!("  FAIL_RATE={}", bench_env.fail_rate);
    eprintln!("  DATA_SCALE={}", bench_env.data_scale);
    eprintln!("  PAYLOAD_VARIANT={}", bench_env.payload_variant);
    eprintln!("  API_URL={}", bench_env.api_url);
    if let Some(seed) = bench_env.seed {
        eprintln!("  SEED={}", seed);
    }
    if bench_env.profile {
        eprintln!("  PROFILE=true");
    }
    eprintln!();

    // Execution steps based on args
    eprintln!("Execution steps (not executed):");
    let mut step = 1;

    if !args.skip_api_start {
        let compose_file = root.join("benches/api/docker/compose.ci.yaml");
        let docker_env_string = bench_env.to_docker_env_string();
        eprintln!("  {}. Start API:", step);
        eprintln!("     {} \\", docker_env_string);
        eprintln!(
            "       docker compose -f {} up -d --build --wait",
            compose_file.display()
        );
        step += 1;
    } else {
        eprintln!("  [SKIP] Start API (--skip-api-start)");
    }

    if !args.skip_health_check {
        eprintln!(
            "  {}. Health check: curl -sf {}/health",
            step, bench_env.api_url
        );
        step += 1;
    } else {
        eprintln!("  [SKIP] Health check (--skip-health-check)");
    }

    if !args.skip_setup {
        let setup_script = root.join("benches/api/benchmarks/setup_test_data.sh");
        let meta_output = root.join("benches/api/benchmarks/setup_meta.json");
        let seed_arg = bench_env
            .seed
            .map(|s| format!(" --seed {}", s))
            .unwrap_or_default();
        eprintln!(
            "  {}. Setup test data: {} --scale {} --payload {}{} --meta-output {}",
            step,
            setup_script.display(),
            bench_env.data_scale,
            bench_env.payload_variant,
            seed_arg,
            meta_output.display()
        );
        step += 1;
    } else {
        eprintln!("  [SKIP] Setup test data (--skip-setup)");
    }

    let benchmark_script = root.join("benches/api/benchmarks/run_benchmark.sh");
    if args.quick {
        eprintln!(
            "  {}. Run benchmark: {} --scenario {} --quick",
            step,
            benchmark_script.display(),
            scenario_path.display()
        );
    } else {
        eprintln!(
            "  {}. Run benchmark: {} --scenario {}",
            step,
            benchmark_script.display(),
            scenario_path.display()
        );
    }
    step += 1;

    if !args.keep_api_running && !args.skip_api_start {
        let compose_file = root.join("benches/api/docker/compose.ci.yaml");
        eprintln!(
            "  {}. Stop API: docker compose -f {} down -v",
            step,
            compose_file.display()
        );
    } else if args.skip_api_start {
        eprintln!("  [SKIP] Stop API (API was not started)");
    } else if args.keep_api_running {
        eprintln!("  [SKIP] Stop API (--keep-api-running)");
    }
    eprintln!();

    eprintln!("=== END DRY RUN ===");
}

pub fn run(args: BenchApiArgs) -> Result<()> {
    let root = project_root()?;

    // Validate scenario file exists
    let scenario_path = if args.scenario.is_absolute() {
        args.scenario.clone()
    } else {
        root.join(&args.scenario)
    };

    // Resolve scenario path - try alternative location if not found
    let scenario_path = if scenario_path.exists() {
        scenario_path
    } else {
        // Try looking in scenarios directory
        let alt_path = root
            .join("benches/api/benchmarks/scenarios")
            .join(&args.scenario);
        if alt_path.exists() {
            alt_path
        } else {
            bail!(
                "Scenario file not found: {} or {}",
                scenario_path.display(),
                alt_path.display()
            );
        }
    };

    // Load scenario configuration
    let scenario_content =
        fs::read_to_string(&scenario_path).context("Failed to read scenario file")?;
    let scenario: ScenarioConfig =
        serde_yaml::from_str(&scenario_content).context("Failed to parse scenario YAML")?;

    // Create environment configuration
    let bench_env = BenchEnv::from_args_and_scenario(&args, &scenario, &root)?;

    eprintln!("==============================================");
    eprintln!("  API Benchmark Runner (xtask)");
    eprintln!("==============================================");
    eprintln!();
    eprintln!("Configuration:");
    eprintln!("  Scenario:       {}", scenario_path.display());
    eprintln!("  Storage Mode:   {}", bench_env.storage_mode);
    eprintln!("  Cache Mode:     {}", bench_env.cache_mode);
    eprintln!("  Cache Strategy: {}", bench_env.cache_strategy);
    eprintln!("  Hit Rate:       {}%", bench_env.hit_rate);
    eprintln!("  Fail Rate:      {}", bench_env.fail_rate);
    eprintln!("  Data Scale:     {}", bench_env.data_scale);
    eprintln!("  Payload:        {}", bench_env.payload_variant);
    if let Some(threads) = bench_env.worker_threads {
        eprintln!("  Worker Threads: {}", threads);
    } else {
        eprintln!("  Worker Threads: (default)");
    }
    if let Some(size) = bench_env.database_pool_size {
        eprintln!("  DB Pool Size:   {}", size);
    } else {
        eprintln!("  DB Pool Size:   (default)");
    }
    if let Some(size) = bench_env.redis_pool_size {
        eprintln!("  Redis Pool:     {}", size);
    } else {
        eprintln!("  Redis Pool:     (default)");
    }
    if let Some(seed) = bench_env.seed {
        eprintln!("  Seed:           {}", seed);
    }
    eprintln!("  Profile:        {}", bench_env.profile);
    eprintln!("  Quick Mode:     {}", args.quick);
    eprintln!("  Dry Run:        {}", args.dry_run);
    eprintln!();

    // Dry-run mode: show commands without executing (no side effects)
    if args.dry_run {
        print_dry_run_output(&bench_env, &scenario_path, &root, &args);
        return Ok(());
    }

    // Set environment variables (only when actually executing)
    bench_env.set_env_vars();

    let mut api_guard = ApiGuard::new(&root);

    if !args.skip_api_start {
        // Always call start_api to ensure environment variables are applied (ENV-REQ-010)
        // start_api handles the case where API is already running by restarting it
        start_api(&root, &bench_env)?;
        api_guard.mark_started();
    }

    if !args.skip_health_check {
        eprintln!("Checking API health...");
        if !check_api_health(&bench_env.api_url, 3)? {
            // On error, let the guard stop the API (don't call keep_running)
            bail!("API health check failed after 3 retries");
        }
        eprintln!("API is healthy");
    }

    if !args.skip_setup {
        setup_test_data(&root, &bench_env)?;
    }

    let results_dir = run_benchmark(&root, &scenario_path, args.quick)?;

    if !args.keep_api_running && api_guard.will_stop() {
        eprintln!("Stopping API...");
        stop_api(&root)?;
        api_guard.keep_running(); // Prevent double-stop from guard
    } else if args.keep_api_running {
        api_guard.keep_running();
    }

    eprintln!();
    eprintln!("==============================================");
    eprintln!("  Benchmark Complete");
    eprintln!("==============================================");
    eprintln!();
    eprintln!("  Results: {}", results_dir.display());
    eprintln!();

    Ok(())
}
