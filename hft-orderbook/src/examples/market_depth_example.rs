//! Example demonstrating market depth analysis functionality

use hft_orderbook::order_book::*;
use hft_orderbook::order_book::market_depth::*;
use hft_orderbook::matching_engine::*;
use std::io::{self, Write};
use colored::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "HFT Order Book Market Depth Analysis Example".bright_green().bold());
    println!("=====================================================\n");
    
    // Create order book and matching engine
    let config = OrderBookConfig::default();
    let book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    let engine_config = MatchingEngineConfig::default();
    let mut engine = MatchingEngine::new(book, engine_config);
    
    // Add some realistic limit orders
    println!("{}", "Creating realistic order book...".bright_blue());
    
    // Populate with bid orders (buys)
    let bid_prices = [
        9950.0, 9960.0, 9970.0, 9980.0, 9990.0, 
        9995.0, 9998.0, 9999.0, 10000.0
    ];
    
    let bid_quantities = [
        10.0, 8.0, 5.0, 3.0, 2.0, 
        1.5, 1.0, 0.5, 0.1
    ];
    
    for i in 0..bid_prices.len() {
        let _ = engine.add_limit_order(
            OrderSide::Buy,
            price_to_internal(bid_prices[i]),
            bid_quantities[i],
            format!("client{}", i),
            TimeInForce::GTC,
        );
    }
    
    // Populate with ask orders (sells)
    let ask_prices = [
        10001.0, 10002.0, 10005.0, 10010.0, 10020.0, 
        10030.0, 10040.0, 10050.0, 10100.0
    ];
    
    let ask_quantities = [
        0.1, 0.5, 1.0, 1.5, 2.0, 
        3.0, 5.0, 8.0, 10.0
    ];
    
    for i in 0..ask_prices.len() {
        let _ = engine.add_limit_order(
            OrderSide::Sell,
            price_to_internal(ask_prices[i]),
            ask_quantities[i],
            format!("client{}", i + 100),
            TimeInForce::GTC,
        );
    }
    
    println!("Order book populated with {} bid levels and {} ask levels\n", 
             bid_prices.len(), ask_prices.len());
    
    // Get book reference for analysis
    let book = engine.get_book();
    
    // 1. Basic Market Depth
    println!("{}", "1. Basic Market Depth Analysis".bright_yellow().bold());
    
    // Get market depth with default options (top 10 levels)
    let depth_data = book.get_market_depth(None);
    
    println!("Market Depth as of {}", depth_data.timestamp);
    println!("\n{}", "BID LEVELS".bright_blue());
    println!("{:<10} {:<10} {:<10} {:<15} {:<10}", 
             "Price", "Volume", "Orders", "Cumulative", "% Total");
    println!("{}", "-".repeat(55));
    
    for level in &depth_data.bids {
        println!("{:<10.2} {:<10.4} {:<10} {:<15.4} {:<10.2}%",
            level.price as f64 / PRICE_MULTIPLIER as f64,
            level.volume,
            level.order_count,
            level.cumulative_volume,
            level.percentage_of_total);
    }
    
    println!("\n{}", "ASK LEVELS".bright_red());
    println!("{:<10} {:<10} {:<10} {:<15} {:<10}", 
             "Price", "Volume", "Orders", "Cumulative", "% Total");
    println!("{}", "-".repeat(55));
    
    for level in &depth_data.asks {
        println!("{:<10.2} {:<10.4} {:<10} {:<15.4} {:<10.2}%",
            level.price as f64 / PRICE_MULTIPLIER as f64,
            level.volume,
            level.order_count,
            level.cumulative_volume,
            level.percentage_of_total);
    }
    
    // 2. Market Depth Statistics
    println!("\n{}", "2. Market Depth Statistics".bright_yellow().bold());
    println!("{:<20} {:<10}", "VWAP:", format!("{:.2}", depth_data.statistics.vwap));
    println!("{:<20} {:<10}", "Spread:", format!("{:.2}", depth_data.statistics.spread));
    println!("{:<20} {:<10}", "Spread (bps):", format!("{:.2}", depth_data.statistics.spread_bps));
    println!("{:<20} {:<10}", "Bid/Ask Ratio:", format!("{:.2}", depth_data.statistics.bid_ask_ratio));
    println!("{:<20} {:<10}", "Imbalance:", format!("{:.2}", depth_data.statistics.imbalance));
    println!("{:<20} {:<10}", "Liquidity Index:", format!("{:.2}", depth_data.statistics.liquidity_index));
    
    // 3. Market Impact Analysis
    println!("\n{}", "3. Market Impact Analysis".bright_yellow().bold());
    
    let order_sizes = [1.0, 5.0, 10.0, 20.0, 50.0];
    
    for &size in &order_sizes {
        println!("\nMarket impact for {} BTC market buy:", size);
        let impact = book.calculate_market_impact(OrderSide::Buy, size);
        
        println!("  Expected execution price: {:.2}", impact.expected_execution_price);
        println!("  Price impact: {:.2}%", impact.expected_price_impact_percent);
        println!("  Slippage: {:.2}%", impact.expected_slippage);
        println!("  Execution cost: {:.2}", impact.estimated_execution_cost);
        
        println!("  Execution levels:");
        for level in &impact.execution_levels {
            println!("    Price: {:.2}, Volume: {:.4}, % of Order: {:.2}%",
                level.price as f64 / PRICE_MULTIPLIER as f64,
                level.executed_volume,
                level.percentage_of_order);
        }
    }
    
    // 4. Liquidity Profile
    println!("\n{}", "4. Liquidity Profile Analysis".bright_yellow().bold());
    
    let ranges = [0.1, 0.5, 1.0, 2.0, 5.0];
    
    for &range in &ranges {
        println!("\nLiquidity profile with {}% range:", range);
        let profile = book.get_liquidity_profile(range);
        
        println!("  Mid price: {:.2}", profile.mid_price);
        println!("  Price range: {}% ({:.2} - {:.2})",
            profile.price_range_percent,
            profile.min_price,
            profile.max_price);
        println!("  Bid volume in range: {:.4}", profile.bid_volume_in_range);
        println!("  Ask volume in range: {:.4}", profile.ask_volume_in_range);
        println!("  Total volume in range: {:.4}", profile.total_volume_in_range);
        println!("  Bid concentration: {:.2}%", profile.bid_concentration * 100.0);
        println!("  Ask concentration: {:.2}%", profile.ask_concentration * 100.0);
    }
    
    // 5. Interactive Example
    println!("\n{}", "5. Interactive Market Impact Calculator".bright_yellow().bold());
    println!("Enter a market order size (or 'q' to quit):");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();
    
    loop {
        print!("> ");
        stdout.flush()?;
        
        input.clear();
        stdin.read_line(&mut input)?;
        
        let input = input.trim();
        
        if input == "q" || input == "quit" {
            break;
        }
        
        match input.parse::<f64>() {
            Ok(size) if size > 0.0 => {
                // Calculate market impact for buy
                let buy_impact = book.calculate_market_impact(OrderSide::Buy, size);
                
                // Calculate market impact for sell
                let sell_impact = book.calculate_market_impact(OrderSide::Sell, size);
                
                println!("\n{}", "BUY IMPACT:".bright_green());
                println!("  Expected execution price: {:.2}", buy_impact.expected_execution_price);
                println!("  Price impact: {:.2}%", buy_impact.expected_price_impact_percent);
                
                println!("\n{}", "SELL IMPACT:".bright_red());
                println!("  Expected execution price: {:.2}", sell_impact.expected_execution_price);
                println!("  Price impact: {:.2}%", sell_impact.expected_price_impact_percent);
                
                println!("\n{}", "ROUND TRIP COST:".bright_yellow());
                let round_trip_cost = (buy_impact.expected_execution_price - 
                                       sell_impact.expected_execution_price) * size;
                println!("  Cost: {:.2}", round_trip_cost);
                println!("  Cost (bps): {:.2}", 
                         round_trip_cost / (size * buy_impact.expected_execution_price) * 10000.0);
            },
            _ => {
                println!("Please enter a valid positive number");
            }
        }
    }
    
    println!("\nThank you for using the Market Depth Analysis Example!");
    
    Ok(())
}

// Implementation of get_book method for MatchingEngine
impl MatchingEngine {
    pub fn get_book(&self) -> &OrderBook {
        // This is a simplified implementation for the example
        // In a real implementation, you would need to handle this properly
        &self.get_book_ref()
    }
    
    // Helper method to get a reference to the order book
    fn get_book_ref(&self) -> &OrderBook {
        // This is a dummy implementation that would need to be properly
        // implemented in the actual MatchingEngine class
        unimplemented!("This is just a placeholder for the example");
    }
}