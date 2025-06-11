//! Benchmarks for the order book implementation.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use hft_orderbook::order_book::*;
use hft_orderbook::matching_engine::*;
use hft_orderbook::simulation::OrderGenerator;
use hft_orderbook::simulation::OrderGeneratorConfig;

fn create_order_book() -> OrderBook {
    let config = OrderBookConfig::default();
    OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config)
}

fn create_matching_engine() -> MatchingEngine {
    let book = create_order_book();
    let config = MatchingEngineConfig::default();
    MatchingEngine::new(book, config)
}

fn bench_add_limit_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_limit_order");
    
    // Benchmark adding limit orders
    let mut order_book = create_order_book();
    group.bench_function("add_limit_order", |b| {
        b.iter(|| {
            let _ = order_book.add_limit_order(
                OrderSide::Buy,
                price_to_internal(10000.0),
                1.0,
                "client1".to_string(),
                TimeInForce::GTC,
            );
        });
    });
    
    group.finish();
}

fn bench_match_orders(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_orders");
    
    // Create a matching engine with some existing orders
    let mut engine = create_matching_engine();
    
    // Add some buy orders
    for i in 0..10 {
        let _ = engine.add_limit_order(
            OrderSide::Buy,
            price_to_internal(10000.0 - i as f64),
            1.0,
            format!("client{}", i),
            TimeInForce::GTC,
        );
    }
    
    // Benchmark matching orders
    group.bench_function("match_orders", |b| {
        b.iter(|| {
            let _ = engine.add_limit_order(
                OrderSide::Sell,
                price_to_internal(10000.0),
                0.1,
                "client_sell".to_string(),
                TimeInForce::IOC,
            );
        });
    });
    
    group.finish();
}

fn bench_order_book_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_book_throughput");
    
    // Create an order generator
    let config = OrderGeneratorConfig::default();
    let mut generator = OrderGenerator::new(config);
    
    // Generate orders
    let num_orders = 1000;
    let orders = generator.generate_orders(num_orders);
    
    // Benchmark order book throughput
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut engine = create_matching_engine();
            
            b.iter(|| {
                for i in 0..size {
                    let order = &orders[i % orders.len()];
                    
                    if order.order_type == OrderType::Limit {
                        let _ = engine.add_limit_order(
                            order.side,
                            order.price,
                            order.remaining_quantity,
                            order.client_id.clone(),
                            order.time_in_force,
                        );
                    } else {
                        let _ = engine.add_market_order(
                            order.side,
                            order.remaining_quantity,
                            order.client_id.clone(),
                        );
                    }
                }
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_add_limit_order,
    bench_match_orders,
    bench_order_book_throughput
);
criterion_main!(benches);
