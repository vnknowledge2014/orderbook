//! Command-line interface for the HFT order book.

use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
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
    Simulate(SimulateArgs),
    
    /// Run a benchmark of the order book
    Benchmark {
        /// Number of iterations
        #[arg(short, long, default_value_t = 1000)]
        iterations: usize,
    },

    /// Run market depth analysis example
    MarketDepth {
        /// Output file for results
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Args)]
struct SimulateArgs {
    /// Load level to simulate
    #[arg(short, long, default_value = "medium")]
    load: String,

    /// Duration of the simulation in seconds
    #[arg(short, long, default_value_t = 60)]
    duration: u64,

    /// Output file for results
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable market depth analysis
    #[arg(long, default_value_t = false)]
    market_depth: bool,

    /// Market depth sample interval in milliseconds
    #[arg(long, default_value_t = 1000)]
    market_depth_interval: u64,

    /// Market impact analysis order sizes (comma-separated values)
    #[arg(long)]
    market_impact_sizes: Option<String>,
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
        Commands::Simulate(args) => {
            let mode = match args.load.to_lowercase().as_str() {
                "low" => SimulationMode::Low,
                "medium" => SimulationMode::Medium,
                "high" => SimulationMode::High,
                "max" => SimulationMode::Max,
                _ => {
                    eprintln!("Invalid load level. Using 'medium'.");
                    SimulationMode::Medium
                }
            };

            // Parse market impact sizes if provided
            let market_impact_sizes = args.market_impact_sizes.map(|s| {
                s.split(',')
                    .filter_map(|size| size.trim().parse::<f64>().ok())
                    .collect::<Vec<f64>>()
            });

            let config = SimulationConfig {
                mode,
                duration_secs: args.duration,
                output_file: args.output,
                market_depth_analysis: args.market_depth,
                market_depth_interval_ms: args.market_depth_interval,
                market_impact_sizes,
            };

            run_simulation(config)?;
        }
        Commands::Benchmark { iterations } => {
            println!("Running benchmark with {} iterations", iterations);
            // Benchmarking code will be implemented separately
        }
        Commands::MarketDepth { output } => {
            println!("Running market depth analysis example");
            
            // Import the example code directly from the examples directory
            // In a real implementation, you might want to move this to a separate module
            let output_path = output.map(|p| p.to_string_lossy().to_string());
            if let Some(path) = &output_path {
                println!("Results will be saved to: {}", path);
            }
            
            // Run the market depth example
            // In a real implementation, you would call a function to run the example
            println!("Please run the example directly: cargo run --example market_depth_example");
        }
    }

    Ok(())
}