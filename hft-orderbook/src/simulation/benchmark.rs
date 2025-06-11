//! Benchmarking utilities for the order book.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use serde::{Serialize, Deserialize};
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;
use crate::order_book::{
    OrderBook, Order, OrderType, OrderBookConfig
};
use crate::order_book::OrderSide;
use crate::order_book::TimeInForce;
use crate::matching_engine::{MatchingEngine, MatchingEngineConfig};
use crate::simulation::OrderGenerator;

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

/// Run a simulation
pub fn run_simulation(config: SimulationConfig) -> anyhow::Result<()> {
    println!("{}", "Running HFT Order Book Simulation".bright_blue().bold());
    println!("Mode: {}", format!("{:?}", config.mode).bright_green());
    println!("Duration: {} seconds", config.duration_secs);
    
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
    let mut order_generator = OrderGenerator::new(order_generator_config.clone());  // Clone here
    
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
                        let latency_micros = elapsed.as_secs() as f64 * 1_000_000.0 + elapsed.subsec_nanos() as f64 / 1_000.0;
                        
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
                    let latency_micros = elapsed.as_secs() as f64 * 1_000_000.0 + elapsed.subsec_nanos() as f64 / 1_000.0;
                    
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
    
    // Save results to file if requested
    if let Some(output_file) = config.output_file {
        let json = serde_json::to_string_pretty(&results)?;
        std::fs::write(output_file, json)?;
    }
    
    Ok(())
}
