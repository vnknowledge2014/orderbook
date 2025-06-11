//! Tests for the matching engine implementation.

use hft_orderbook::order_book::*;
use hft_orderbook::matching_engine::*;

#[test]
fn test_matching_engine_add_limit_order() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Create a matching engine
    let engine_config = MatchingEngineConfig::default();
    let mut engine = MatchingEngine::new(book, engine_config);
    
    // Add a buy order
    let result = engine.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    );
    
    assert!(result.is_ok());
    let match_result = result.unwrap();
    assert!(match_result.trades.is_empty());
    assert!(match_result.remaining_order.is_some());
}

#[test]
fn test_matching_engine_match_orders() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Create a matching engine
    let engine_config = MatchingEngineConfig::default();
    let mut engine = MatchingEngine::new(book, engine_config);
    
    // Add a buy order
    let buy_result = engine.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add a matching sell order
    let sell_result = engine.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10000.0),
        1.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Check that a trade was generated
    assert_eq!(sell_result.trades.len(), 1);
    let trade = &sell_result.trades[0];
    assert_eq!(trade.price, price_to_internal(10000.0));
    assert_eq!(trade.quantity, 1.0);
    assert_eq!(trade.aggressor_side, OrderSide::Sell);
}

#[test]
fn test_matching_engine_partial_match() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Create a matching engine
    let engine_config = MatchingEngineConfig::default();
    let mut engine = MatchingEngine::new(book, engine_config);
    
    // Add a buy order
    let buy_result = engine.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        2.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add a smaller matching sell order
    let sell_result = engine.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10000.0),
        1.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Check that a trade was generated
    assert_eq!(sell_result.trades.len(), 1);
    let trade = &sell_result.trades[0];
    assert_eq!(trade.price, price_to_internal(10000.0));
    assert_eq!(trade.quantity, 1.0);
    
    // Check that the remaining buy order is still in the book
    let buy_order = engine.cancel_order(&buy_result.remaining_order.unwrap().id).unwrap();
    assert_eq!(buy_order.remaining_quantity, 1.0);
}

#[test]
fn test_matching_engine_market_order() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Create a matching engine
    let engine_config = MatchingEngineConfig::default();
    let mut engine = MatchingEngine::new(book, engine_config);
    
    // Add a limit order
    let limit_result = engine.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add a matching market order
    let market_result = engine.add_market_order(
        OrderSide::Sell,
        1.0,
        "client2".to_string(),
    ).unwrap();
    
    // Check that a trade was generated
    assert_eq!(market_result.trades.len(), 1);
    let trade = &market_result.trades[0];
    assert_eq!(trade.price, price_to_internal(10000.0));
    assert_eq!(trade.quantity, 1.0);
    assert_eq!(trade.aggressor_side, OrderSide::Sell);
}
