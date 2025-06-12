//! Benchmarking utilities for the order book.

use crate::matching_engine::{MatchingEngine, MatchingEngineConfig};
use crate::order_book::market_depth::{MarketDepthData, MarketDepthOptions, MarketImpactData};
use crate::order_book::{Order, OrderBook, OrderBookConfig, OrderSide, OrderType};
use crate::simulation::OrderGenerator;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Simulation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationMode {
    /// Low load (1,000 orders per second)
    Low,
    /// Medium load (10,000 orders per second)
    Medium,
    /// High load (100,000 orders per second)
    High,
    /// Maximum load (as fast as possible)
    Max,
}

/// Simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Simulation mode
    pub mode: SimulationMode,
    /// Duration of the simulation in seconds
    pub duration_secs: u64,
    /// Output file for results
    pub output_file: Option<PathBuf>,
    /// Enable market depth analysis
    pub market_depth_analysis: bool,
    /// Market depth sample interval (in milliseconds)
    pub market_depth_interval_ms: u64,
    /// Market impact analysis order sizes to test
    pub market_impact_sizes: Option<Vec<f64>>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            mode: SimulationMode::Medium,
            duration_secs: 60,
            output_file: None,
            market_depth_analysis: false,
            market_depth_interval_ms: 1000,
            market_impact_sizes: None,
        }
    }
}

/// Simulation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResults {
    /// Simulation mode
    pub mode: String,
    /// Duration of the simulation
    pub duration: f64,
    /// Number of orders processed
    pub orders_processed: usize,
    /// Orders per second
    pub orders_per_second: f64,
    /// Latency statistics (in microseconds)
    pub latency: LatencyStatistics,
    /// Memory usage (in megabytes)
    pub memory_usage: f64,
    /// Market depth analysis (if enabled)
    pub market_depth: Option<MarketDepthAnalysis>,
    /// Market impact analysis (if enabled)
    pub market_impact: Option<MarketImpactAnalysis>,
}

/// Latency statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStatistics {
    /// Minimum latency
    pub min: f64,
    /// Maximum latency
    pub max: f64,
    /// Average latency
    pub avg: f64,
    /// 50th percentile latency
    pub p50: f64,
    /// 99th percentile latency
    pub p99: f64,
}

/// Market depth analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepthAnalysis {
    /// Timestamp of the analysis
    pub timestamp: u64,
    /// Bid-ask spread statistics
    pub spread_stats: SpreadStatistics,
    /// Liquidity statistics
    pub liquidity_stats: LiquidityStatistics,
    /// Imbalance statistics
    pub imbalance_stats: ImbalanceStatistics,
    /// Final market depth snapshot
    pub final_depth: MarketDepthData,
}

/// Spread statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadStatistics {
    /// Minimum spread
    pub min: f64,
    /// Maximum spread
    pub max: f64,
    /// Average spread
    pub avg: f64,
    /// Minimum spread in basis points
    pub min_bps: f64,
    /// Maximum spread in basis points
    pub max_bps: f64,
    /// Average spread in basis points
    pub avg_bps: f64,
}

/// Liquidity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityStatistics {
    /// Total bid volume
    pub total_bid_volume: f64,
    /// Total ask volume
    pub total_ask_volume: f64,
    /// Average bid volume per level
    pub avg_bid_volume_per_level: f64,
    /// Average ask volume per level
    pub avg_ask_volume_per_level: f64,
    /// Liquidity concentration (% of volume in top 3 levels)
    pub top_level_concentration: f64,
}

/// Imbalance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImbalanceStatistics {
    /// Minimum imbalance
    pub min: f64,
    /// Maximum imbalance
    pub max: f64,
    /// Average imbalance
    pub avg: f64,
    /// Bid-ask ratio average
    pub bid_ask_ratio_avg: f64,
}

/// Market impact analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketImpactAnalysis {
    /// Timestamp of the analysis
    pub timestamp: u64,
    /// Order sizes analyzed
    pub order_sizes: Vec<f64>,
    /// Buy impact analysis
    pub buy_impact: Vec<MarketImpactData>,
    /// Sell impact analysis
    pub sell_impact: Vec<MarketImpactData>,
}

/// Run a simulation
pub fn run_simulation(config: SimulationConfig) -> anyhow::Result<()> {
    println!(
        "{}",
        "Running HFT Order Book Simulation".bright_blue().bold()
    );
    println!("Mode: {}", format!("{:?}", config.mode).bright_green());
    println!("Duration: {} seconds", config.duration_secs);

    if config.market_depth_analysis {
        println!("Market Depth Analysis: {}", "Enabled".bright_green());
        println!(
            "Market Depth Sample Interval: {} ms",
            config.market_depth_interval_ms
        );
    } else {
        println!("Market Depth Analysis: {}", "Disabled".bright_red());
    }

    if let Some(ref sizes) = config.market_impact_sizes {
        println!("Market Impact Analysis: {}", "Enabled".bright_green());
        println!("Order Sizes: {:?}", sizes);
    } else {
        println!("Market Impact Analysis: {}", "Disabled".bright_red());
    }

    // Create order book
    let order_book_config = OrderBookConfig::default();

    let order_book = OrderBook::new(
        "BTCUSD".to_string(),
        "EXCHANGE".to_string(),
        order_book_config,
    );

    // Create matching engine
    let matching_engine_config = MatchingEngineConfig::default();
    let matching_engine = Arc::new(Mutex::new(MatchingEngine::new(
        order_book,
        matching_engine_config,
    )));

    // Create order generator
    let order_generator_config = crate::simulation::generator::OrderGeneratorConfig::default();
    let mut order_generator = OrderGenerator::new(order_generator_config.clone());

    // Determine order rate based on mode
    let orders_per_second = match config.mode {
        SimulationMode::Low => 1_000,
        SimulationMode::Medium => 10_000,
        SimulationMode::High => 100_000,
        SimulationMode::Max => 1_000_000, // Will be limited by system performance
    };

    // Calculate total orders to generate
    let total_orders = orders_per_second * config.duration_secs as usize;

    // Create progress bar
    let progress_bar = ProgressBar::new(total_orders as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    // Track metrics
    let orders_processed = Arc::new(AtomicUsize::new(0));
    let latencies = Arc::new(Mutex::new(Vec::<f64>::new()));

    // For market depth analysis
    let market_depth_samples = if config.market_depth_analysis {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };

    // Start timing
    let start_time = Instant::now();

    // Determine number of threads based on CPU cores
    let num_threads = num_cpus::get();
    println!("Using {} threads", num_threads);

    // Generate orders upfront for max mode, or generate on-the-fly for other modes
    let pre_generated_orders = if config.mode == SimulationMode::Max {
        println!("Generating {} orders...", total_orders);
        let orders = order_generator.generate_orders(total_orders);
        println!("Orders generated");
        Some(Arc::new(orders))
    } else {
        None
    };

    // Set up market depth analysis if enabled
    if config.market_depth_analysis {
        let market_depth_interval = Duration::from_millis(config.market_depth_interval_ms);
        let market_depth_samples = market_depth_samples.as_ref().unwrap().clone();
        let matching_engine = Arc::clone(&matching_engine);
        let duration_secs = config.duration_secs;

        thread::spawn(move || {
            let mut last_sample = Instant::now();

            while last_sample.elapsed() < Duration::from_secs(duration_secs) {
                // Wait for the next sample interval
                thread::sleep(market_depth_interval.saturating_sub(last_sample.elapsed()));

                // Get market depth
                let engine = matching_engine.lock().unwrap();
                let book = engine.get_book();

                // Sample market depth with all statistics
                let options = MarketDepthOptions {
                    max_levels: 10,
                    calculate_statistics: true,
                    calculate_cumulative: true,
                    group_levels: false,
                    group_threshold: None,
                };

                let depth_data = book.get_market_depth(Some(options));

                // Store the sample
                market_depth_samples.lock().unwrap().push(depth_data);

                last_sample = Instant::now();
            }
        });
    }

    // Create threads
    let mut handles = Vec::with_capacity(num_threads);

    // Make a copy of order_generator_config for threads to use
    let thread_config = Arc::new(order_generator_config);

    for thread_id in 0..num_threads {
        let matching_engine = Arc::clone(&matching_engine);
        let orders_processed = Arc::clone(&orders_processed);
        let latencies = Arc::clone(&latencies);
        let progress_bar = progress_bar.clone();
        let pre_generated_orders = pre_generated_orders.clone();
        let thread_config = Arc::clone(&thread_config);

        let thread_total_orders = total_orders / num_threads;
        let thread_orders_per_second = orders_per_second / num_threads;

        let handle = thread::spawn(move || {
            let mut local_order_generator = OrderGenerator::new((*thread_config).clone());

            if config.mode == SimulationMode::Max {
                // Process pre-generated orders as fast as possible
                if let Some(orders) = pre_generated_orders {
                    let start_idx = thread_id * thread_total_orders;
                    let end_idx = start_idx + thread_total_orders;

                    for i in start_idx..end_idx {
                        if i >= orders.len() {
                            break;
                        }

                        let order = orders[i].clone();
                        let start = Instant::now();

                        let mut engine = matching_engine.lock().unwrap();
                        match order.order_type {
                            OrderType::Limit => {
                                let _ = engine.add_limit_order(
                                    order.side,
                                    order.price,
                                    order.remaining_quantity,
                                    order.client_id.clone(),
                                    order.time_in_force,
                                );
                            }
                            OrderType::Market => {
                                let _ = engine.add_market_order(
                                    order.side,
                                    order.remaining_quantity,
                                    order.client_id.clone(),
                                );
                            }
                            _ => {}
                        }

                        let elapsed = start.elapsed();
                        let latency_micros = elapsed.as_secs() as f64 * 1_000_000.0
                            + elapsed.subsec_nanos() as f64 / 1_000.0;

                        latencies.lock().unwrap().push(latency_micros);
                        orders_processed.fetch_add(1, Ordering::Relaxed);
                        progress_bar.inc(1);
                    }
                }
            } else {
                // Generate and process orders at the specified rate
                let mut last_time = Instant::now();
                let mut orders_this_second = 0;

                for _ in 0..thread_total_orders {
                    // Generate a random order
                    let order = local_order_generator.generate_order();

                    // Process the order
                    let start = Instant::now();

                    let mut engine = matching_engine.lock().unwrap();
                    match order.order_type {
                        OrderType::Limit => {
                            let _ = engine.add_limit_order(
                                order.side,
                                order.price,
                                order.remaining_quantity,
                                order.client_id.clone(),
                                order.time_in_force,
                            );
                        }
                        OrderType::Market => {
                            let _ = engine.add_market_order(
                                order.side,
                                order.remaining_quantity,
                                order.client_id.clone(),
                            );
                        }
                        _ => {}
                    }

                    let elapsed = start.elapsed();
                    let latency_micros = elapsed.as_secs() as f64 * 1_000_000.0
                        + elapsed.subsec_nanos() as f64 / 1_000.0;

                    latencies.lock().unwrap().push(latency_micros);
                    orders_processed.fetch_add(1, Ordering::Relaxed);
                    progress_bar.inc(1);

                    // Rate limiting
                    orders_this_second += 1;
                    if orders_this_second >= thread_orders_per_second {
                        let elapsed = last_time.elapsed();
                        if elapsed < Duration::from_secs(1) {
                            thread::sleep(Duration::from_secs(1) - elapsed);
                        }
                        last_time = Instant::now();
                        orders_this_second = 0;
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;

    // Calculate results
    let orders_processed = orders_processed.load(Ordering::Relaxed);
    let orders_per_second = orders_processed as f64 / elapsed_secs;

    // Calculate latency statistics
    let mut latencies = latencies.lock().unwrap();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min_latency = *latencies.first().unwrap_or(&0.0);
    let max_latency = *latencies.last().unwrap_or(&0.0);
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;

    let p50_idx = (latencies.len() as f64 * 0.5) as usize;
    let p99_idx = (latencies.len() as f64 * 0.99) as usize;

    let p50_latency = latencies.get(p50_idx).copied().unwrap_or(0.0);
    let p99_latency = latencies.get(p99_idx).copied().unwrap_or(0.0);

    // Estimate memory usage (rough approximation)
    let memory_usage = (orders_processed * std::mem::size_of::<Order>()) as f64 / (1024.0 * 1024.0);

    // Analyze market depth if enabled
    let market_depth_analysis = if config.market_depth_analysis {
        let samples = market_depth_samples.as_ref().unwrap().lock().unwrap();

        if samples.is_empty() {
            None
        } else {
            // Calculate spread statistics
            let mut min_spread: f64 = f64::MAX;
            let mut max_spread: f64 = 0.0;
            let mut total_spread: f64 = 0.0;
            let mut min_spread_bps: f64 = f64::MAX;
            let mut max_spread_bps: f64 = 0.0;
            let mut total_spread_bps: f64 = 0.0;

            // Calculate liquidity statistics
            let mut total_bid_volume: f64 = 0.0;
            let mut total_ask_volume: f64 = 0.0;
            let mut total_bid_levels: usize = 0;
            let mut total_ask_levels: usize = 0;
            let mut total_top_concentration: f64 = 0.0;

            // Calculate imbalance statistics
            let mut min_imbalance: f64 = 1.0;
            let mut max_imbalance: f64 = -1.0;
            let mut total_imbalance: f64 = 0.0;
            let mut total_bid_ask_ratio: f64 = 0.0;

            for sample in samples.iter() {
                // Spread stats
                let spread = sample.statistics.spread;
                min_spread = min_spread.min(spread);
                max_spread = max_spread.max(spread);
                total_spread += spread;
                
                let spread_bps = sample.statistics.spread_bps;
                min_spread_bps = min_spread_bps.min(spread_bps);
                max_spread_bps = max_spread_bps.max(spread_bps);
                total_spread_bps += spread_bps;
                
                // Liquidity stats
                let bid_volume: f64 = sample.bids.iter().map(|level| level.volume).sum();
                let ask_volume: f64 = sample.asks.iter().map(|level| level.volume).sum();
                
                total_bid_volume += bid_volume;
                total_ask_volume += ask_volume;
                total_bid_levels += sample.bids.len();
                total_ask_levels += sample.asks.len();
                
                // Top level concentration (top 3 levels)
                let top_bid_volume: f64 = sample.bids.iter().take(3).map(|level| level.volume).sum();
                let top_ask_volume: f64 = sample.asks.iter().take(3).map(|level| level.volume).sum();
                let top_concentration = (top_bid_volume + top_ask_volume) / (bid_volume + ask_volume);
                total_top_concentration += top_concentration;
                
                // Imbalance stats
                let imbalance = sample.statistics.imbalance;
                min_imbalance = min_imbalance.min(imbalance);
                max_imbalance = max_imbalance.max(imbalance);
                total_imbalance += imbalance;
                
                total_bid_ask_ratio += sample.statistics.bid_ask_ratio;
            }

            let num_samples = samples.len() as f64;

            let avg_spread = total_spread / num_samples;
            let avg_spread_bps = total_spread_bps / num_samples;

            let avg_bid_volume_per_level = if total_bid_levels > 0 {
                total_bid_volume / total_bid_levels as f64
            } else {
                0.0
            };

            let avg_ask_volume_per_level = if total_ask_levels > 0 {
                total_ask_volume / total_ask_levels as f64
            } else {
                0.0
            };

            let avg_top_concentration = total_top_concentration / num_samples;

            let avg_imbalance = total_imbalance / num_samples;
            let avg_bid_ask_ratio = total_bid_ask_ratio / num_samples;

            Some(MarketDepthAnalysis {
                timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
                spread_stats: SpreadStatistics {
                    min: min_spread,
                    max: max_spread,
                    avg: avg_spread,
                    min_bps: min_spread_bps,
                    max_bps: max_spread_bps,
                    avg_bps: avg_spread_bps,
                },
                liquidity_stats: LiquidityStatistics {
                    total_bid_volume: total_bid_volume / num_samples,
                    total_ask_volume: total_ask_volume / num_samples,
                    avg_bid_volume_per_level,
                    avg_ask_volume_per_level,
                    top_level_concentration: avg_top_concentration,
                },
                imbalance_stats: ImbalanceStatistics {
                    min: min_imbalance,
                    max: max_imbalance,
                    avg: avg_imbalance,
                    bid_ask_ratio_avg: avg_bid_ask_ratio,
                },
                final_depth: samples.last().unwrap().clone(),
            })
        }
    } else {
        None
    };

    // Perform market impact analysis if enabled
    let market_impact_analysis = if let Some(order_sizes) = &config.market_impact_sizes {
        let engine = matching_engine.lock().unwrap();
        let book = engine.get_book();

        let mut buy_impact = Vec::new();
        let mut sell_impact = Vec::new();

        for &size in order_sizes {
            // Calculate buy impact
            let buy = book.calculate_market_impact(OrderSide::Buy, size);
            buy_impact.push(buy);

            // Calculate sell impact
            let sell = book.calculate_market_impact(OrderSide::Sell, size);
            sell_impact.push(sell);
        }

        Some(MarketImpactAnalysis {
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            order_sizes: order_sizes.clone(),
            buy_impact,
            sell_impact,
        })
    } else {
        None
    };

    // Create results
    let results = SimulationResults {
        mode: format!("{:?}", config.mode),
        duration: elapsed_secs,
        orders_processed,
        orders_per_second,
        latency: LatencyStatistics {
            min: min_latency,
            max: max_latency,
            avg: avg_latency,
            p50: p50_latency,
            p99: p99_latency,
        },
        memory_usage,
        market_depth: market_depth_analysis,
        market_impact: market_impact_analysis,
    };

    // Display results
    progress_bar.finish_with_message("Simulation complete");

    println!("\n{}", "Simulation Results".bright_blue().bold());
    println!("Duration: {:.2} seconds", elapsed_secs);
    println!("Orders Processed: {}", orders_processed);
    println!("Orders Per Second: {:.2}", orders_per_second);
    println!("\n{}", "Latency (microseconds)".bright_blue());
    println!("Min: {:.2}", min_latency);
    println!("Max: {:.2}", max_latency);
    println!("Avg: {:.2}", avg_latency);
    println!("P50: {:.2}", p50_latency);
    println!("P99: {:.2}", p99_latency);
    println!("\nEstimated Memory Usage: {:.2} MB", memory_usage);

    // Display market depth analysis results if available
    if let Some(depth_analysis) = &results.market_depth {
        println!("\n{}", "Market Depth Analysis".bright_blue().bold());

        println!("\n{}", "Spread Statistics".bright_yellow());
        println!("Min Spread: {:.2}", depth_analysis.spread_stats.min);
        println!("Max Spread: {:.2}", depth_analysis.spread_stats.max);
        println!("Avg Spread: {:.2}", depth_analysis.spread_stats.avg);
        println!(
            "Min Spread (bps): {:.2}",
            depth_analysis.spread_stats.min_bps
        );
        println!(
            "Max Spread (bps): {:.2}",
            depth_analysis.spread_stats.max_bps
        );
        println!(
            "Avg Spread (bps): {:.2}",
            depth_analysis.spread_stats.avg_bps
        );

        println!("\n{}", "Liquidity Statistics".bright_yellow());
        println!(
            "Avg Bid Volume: {:.4}",
            depth_analysis.liquidity_stats.total_bid_volume
        );
        println!(
            "Avg Ask Volume: {:.4}",
            depth_analysis.liquidity_stats.total_ask_volume
        );
        println!(
            "Avg Bid Volume per Level: {:.4}",
            depth_analysis.liquidity_stats.avg_bid_volume_per_level
        );
        println!(
            "Avg Ask Volume per Level: {:.4}",
            depth_analysis.liquidity_stats.avg_ask_volume_per_level
        );
        println!(
            "Top Level Concentration: {:.2}%",
            depth_analysis.liquidity_stats.top_level_concentration * 100.0
        );

        println!("\n{}", "Imbalance Statistics".bright_yellow());
        println!("Min Imbalance: {:.4}", depth_analysis.imbalance_stats.min);
        println!("Max Imbalance: {:.4}", depth_analysis.imbalance_stats.max);
        println!("Avg Imbalance: {:.4}", depth_analysis.imbalance_stats.avg);
        println!(
            "Avg Bid/Ask Ratio: {:.4}",
            depth_analysis.imbalance_stats.bid_ask_ratio_avg
        );
    }

    // Display market impact analysis results if available
    if let Some(impact_analysis) = &results.market_impact {
        println!("\n{}", "Market Impact Analysis".bright_blue().bold());

        for i in 0..impact_analysis.order_sizes.len() {
            let size = impact_analysis.order_sizes[i];
            let buy = &impact_analysis.buy_impact[i];
            let sell = &impact_analysis.sell_impact[i];

            println!("\nOrder Size: {:.2} BTC", size);
            println!("  Buy Execution Price: {:.2}", buy.expected_execution_price);
            println!(
                "  Buy Price Impact: {:.2}%",
                buy.expected_price_impact_percent
            );
            println!(
                "  Sell Execution Price: {:.2}",
                sell.expected_execution_price
            );
            println!(
                "  Sell Price Impact: {:.2}%",
                sell.expected_price_impact_percent
            );
            println!(
                "  Round-trip Cost: {:.2}",
                (buy.expected_execution_price - sell.expected_execution_price) * size
            );
        }
    }

    // Save results to file if requested
    if let Some(output_file) = config.output_file {
        let json = serde_json::to_string_pretty(&results)?;
        std::fs::write(output_file, json)?;
    }

    Ok(())
}

// REMOVED the duplicate get_book implementation from benchmark.rs
