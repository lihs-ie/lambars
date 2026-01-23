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
    workers: Option<u32>,
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
    workers: u32,
    database_pool_size: u32,
    redis_pool_size: u32,
    data_scale: String,
    payload_variant: String,
    seed: Option<u64>,
    api_url: String,
    profile: bool,
}

impl BenchEnv {
    /// Create environment from scenario config and CLI overrides
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

        // Workers: from concurrency.workers, worker_config.worker_threads, or default
        let workers = env::var("WORKERS")
            .ok()
            .and_then(|v| v.parse().ok())
            .or_else(|| env_vars.get("WORKERS").and_then(|v| v.parse().ok()))
            .or_else(|| scenario.concurrency.as_ref().and_then(|c| c.workers))
            .or_else(|| {
                scenario
                    .worker_config
                    .as_ref()
                    .and_then(|w| w.worker_threads)
            })
            .unwrap_or(4);

        // Pool sizes: from concurrency or pool_sizes
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
            })
            .unwrap_or(16);

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
            .or_else(|| scenario.pool_sizes.as_ref().and_then(|p| p.redis_pool_size))
            .unwrap_or(8);

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
            workers,
            database_pool_size,
            redis_pool_size,
            data_scale,
            payload_variant,
            seed,
            api_url,
            profile: args.profile,
        })
    }

    /// Set environment variables for child processes
    ///
    /// # Safety
    /// This function uses `env::set_var` which is unsafe in Rust 2024 edition.
    /// It should only be called from a single-threaded context before spawning
    /// child processes. The xtask runner is single-threaded, so this is safe.
    fn set_env_vars(&self) {
        // SAFETY: xtask is single-threaded and we set these before spawning
        // any child processes. The environment is inherited by child processes.
        unsafe {
            env::set_var("STORAGE_MODE", &self.storage_mode);
            env::set_var("CACHE_MODE", &self.cache_mode);
            env::set_var("CACHE_STRATEGY", &self.cache_strategy);
            env::set_var("HIT_RATE", self.hit_rate.to_string());
            env::set_var("FAIL_RATE", self.fail_rate.to_string());
            env::set_var("WORKERS", self.workers.to_string());
            env::set_var("DATABASE_POOL_SIZE", self.database_pool_size.to_string());
            env::set_var("REDIS_POOL_SIZE", self.redis_pool_size.to_string());
            env::set_var("DATA_SCALE", &self.data_scale);
            env::set_var("PAYLOAD_VARIANT", &self.payload_variant);
            env::set_var("API_URL", &self.api_url);

            if let Some(seed) = self.seed {
                env::set_var("SEED", seed.to_string());
            }

            if self.profile {
                env::set_var("PROFILE", "true");
            }
        }
    }
}

/// Load environment variables from a file
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

/// Get the project root directory
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

/// Check if API is healthy
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

/// Start API using docker compose
fn start_api(root: &Path) -> Result<()> {
    let compose_file = root.join("benches/api/docker/compose.ci.yaml");

    if !compose_file.exists() {
        bail!("Docker compose file not found: {}", compose_file.display());
    }

    eprintln!("Starting API with docker compose...");

    let status = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_file.to_str().unwrap(),
            "up",
            "-d",
            "--build",
            "--wait",
        ])
        .current_dir(root)
        .status()
        .context("Failed to start docker compose")?;

    if !status.success() {
        // Clean up any partially started containers before returning error
        let _ = stop_api(root);
        bail!("Failed to start API via docker compose");
    }

    Ok(())
}

/// Stop API using docker compose
fn stop_api(root: &Path) -> Result<()> {
    let compose_file = root.join("benches/api/docker/compose.ci.yaml");

    if !compose_file.exists() {
        return Ok(());
    }

    eprintln!("Stopping API...");

    let _ = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_file.to_str().unwrap(),
            "down",
            "-v",
        ])
        .current_dir(root)
        .status();

    Ok(())
}

/// Run setup_test_data.sh
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

/// Run the benchmark using run_benchmark.sh
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

/// Main entry point for bench-api command
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

    // Set environment variables
    bench_env.set_env_vars();

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
    eprintln!("  Workers:        {}", bench_env.workers);
    eprintln!("  DB Pool Size:   {}", bench_env.database_pool_size);
    eprintln!("  Redis Pool:     {}", bench_env.redis_pool_size);
    if let Some(seed) = bench_env.seed {
        eprintln!("  Seed:           {}", seed);
    }
    eprintln!("  Profile:        {}", bench_env.profile);
    eprintln!("  Quick Mode:     {}", args.quick);
    eprintln!();

    // Track whether we started the API (for cleanup on error)
    let mut api_started = false;

    // Step 1: Start API (if not skipped)
    if !args.skip_api_start {
        // First check if API is already running
        if !args.skip_health_check && check_api_health(&bench_env.api_url, 1)? {
            eprintln!("API already running at {}", bench_env.api_url);
        } else {
            start_api(&root)?;
            api_started = true;
        }
    }

    // Helper closure for cleanup on error
    let cleanup_on_error = |root: &Path, should_stop: bool| {
        if should_stop {
            let _ = stop_api(root);
        }
    };

    // Step 2: Health check (if not skipped)
    if !args.skip_health_check {
        eprintln!("Checking API health...");
        if !check_api_health(&bench_env.api_url, 3)? {
            cleanup_on_error(&root, api_started && !args.keep_api_running);
            bail!("API health check failed after 3 retries");
        }
        eprintln!("API is healthy");
    }

    // Step 3: Setup test data (if not skipped)
    if !args.skip_setup
        && let Err(e) = setup_test_data(&root, &bench_env)
    {
        cleanup_on_error(&root, api_started && !args.keep_api_running);
        return Err(e);
    }

    // Step 4: Run benchmark
    let results_dir = match run_benchmark(&root, &scenario_path, args.quick) {
        Ok(dir) => dir,
        Err(e) => {
            cleanup_on_error(&root, api_started && !args.keep_api_running);
            return Err(e);
        }
    };

    // Step 5: Stop API (if not keeping running)
    if !args.keep_api_running && api_started {
        stop_api(&root)?;
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
