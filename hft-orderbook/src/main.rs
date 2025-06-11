//! Command-line interface for the HFT order book.

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use hft_orderbook::simulation::{run_simulation, SimulationConfig, SimulationMode};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a simulation with the order book
    Simulate {
        /// Load level to simulate
        #[arg(short, long, default_value = "medium")]
        load: String,

        /// Duration of the simulation in seconds
        #[arg(short, long, default_value_t = 60)]
        duration: u64,

        /// Output file for results
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Run a benchmark of the order book
    Benchmark {
        /// Number of iterations
        #[arg(short, long, default_value_t = 1000)]
        iterations: usize,
    },
}

fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Simulate { load, duration, output } => {
            let mode = match load.to_lowercase().as_str() {
                "low" => SimulationMode::Low,
                "medium" => SimulationMode::Medium,
                "high" => SimulationMode::High,
                "max" => SimulationMode::Max,
                _ => {
                    eprintln!("Invalid load level. Using 'medium'.");
                    SimulationMode::Medium
                }
            };

            let config = SimulationConfig {
                mode,
                duration_secs: duration,
                output_file: output,
            };

            run_simulation(config)?;
        }
        Commands::Benchmark { iterations } => {
            println!("Running benchmark with {} iterations", iterations);
            // Benchmarking code will be implemented separately
        }
    }

    Ok(())
}
