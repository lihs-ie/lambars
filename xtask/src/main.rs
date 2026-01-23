//! xtask - Development task runner for lambars
//!
//! Usage:
//!   cargo xtask bench-api --scenario <yaml> [options]

mod bench_api;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development task runner for lambars")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run API benchmarks with scenario configuration
    BenchApi(bench_api::BenchApiArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BenchApi(args) => bench_api::run(args),
    }
}
