//! Tests for the order book implementation.

use hft_orderbook::order_book::*;

#[test]
fn test_order_book_add_limit_order() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add a buy order
    let order_id = book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Check that the order was added
    let best_bid = book.best_bid();
    assert!(best_bid.is_some());
    assert_eq!(best_bid.unwrap(), price_to_internal(10000.0));
    
    // Add a sell order
    let order_id = book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10001.0),
        1.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Check that the order was added
    let best_ask = book.best_ask();
    assert!(best_ask.is_some());
    assert_eq!(best_ask.unwrap(), price_to_internal(10001.0));
    
    // Check the spread
    let spread = book.spread();
    assert!(spread.is_some());
    assert_eq!(spread.unwrap(), price_to_internal(10001.0) - price_to_internal(10000.0));
}

#[test]
fn test_order_book_cancel_order() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add a buy order
    let order_id = book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Cancel the order
    let order = book.cancel_order(&order_id);
    assert!(order.is_ok());
    
    // Check that the order was cancelled
    let best_bid = book.best_bid();
    assert!(best_bid.is_none());
}

#[test]
fn test_order_book_top_bids_asks() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add multiple buy orders at different prices
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(9999.0),
        2.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(9998.0),
        3.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add multiple sell orders at different prices
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10001.0),
        1.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10002.0),
        2.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10003.0),
        3.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Get top bids
    let top_bids = book.top_bids(3);
    assert_eq!(top_bids.len(), 3);
    assert_eq!(top_bids[0].0, price_to_internal(10000.0));
    assert_eq!(top_bids[1].0, price_to_internal(9999.0));
    assert_eq!(top_bids[2].0, price_to_internal(9998.0));
    
    // Get top asks
    let top_asks = book.top_asks(3);
    assert_eq!(top_asks.len(), 3);
    assert_eq!(top_asks[0].0, price_to_internal(10001.0));
    assert_eq!(top_asks[1].0, price_to_internal(10002.0));
    assert_eq!(top_asks[2].0, price_to_internal(10003.0));
}
