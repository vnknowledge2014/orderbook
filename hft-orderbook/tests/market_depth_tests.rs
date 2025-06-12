//! Tests for the market depth functionality.

use hft_orderbook::order_book::*;
use hft_orderbook::order_book::market_depth::*;

#[test]
fn test_market_depth_basic() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add some bid orders
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(9990.0),
        2.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add some ask orders
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10010.0),
        1.5,
        "client3".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10020.0),
        2.5,
        "client4".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Get market depth with default options
    let depth_data = book.get_market_depth(None);
    
    // Verify bids
    assert_eq!(depth_data.bids.len(), 2);
    assert_eq!(depth_data.bids[0].price, price_to_internal(10000.0));
    assert_eq!(depth_data.bids[0].volume, 1.0);
    assert_eq!(depth_data.bids[1].price, price_to_internal(9990.0));
    assert_eq!(depth_data.bids[1].volume, 2.0);
    
    // Verify asks
    assert_eq!(depth_data.asks.len(), 2);
    assert_eq!(depth_data.asks[0].price, price_to_internal(10010.0));
    assert_eq!(depth_data.asks[0].volume, 1.5);
    assert_eq!(depth_data.asks[1].price, price_to_internal(10020.0));
    assert_eq!(depth_data.asks[1].volume, 2.5);
    
    // Verify statistics are calculated
    assert!(depth_data.statistics.spread > 0.0);
    assert!(depth_data.statistics.spread_bps > 0.0);
    assert!(depth_data.statistics.bid_ask_ratio > 0.0);
}

#[test]
fn test_market_depth_options() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add multiple bid orders
    for i in 0..20 {
        book.add_limit_order(
            OrderSide::Buy,
            price_to_internal(10000.0 - i as f64 * 10.0),
            1.0 + i as f64 * 0.1,
            format!("client{}", i),
            TimeInForce::GTC,
        ).unwrap();
    }
    
    // Add multiple ask orders
    for i in 0..20 {
        book.add_limit_order(
            OrderSide::Sell,
            price_to_internal(10010.0 + i as f64 * 10.0),
            1.5 + i as f64 * 0.1,
            format!("client{}", i + 100),
            TimeInForce::GTC,
        ).unwrap();
    }
    
    // Get market depth with custom options (limit to 5 levels)
    let options = MarketDepthOptions {
        max_levels: 5,
        calculate_statistics: true,
        calculate_cumulative: true,
        group_levels: false,
        group_threshold: None,
    };
    
    let depth_data = book.get_market_depth(Some(options));
    
    // Verify number of levels is limited
    assert_eq!(depth_data.bids.len(), 5);
    assert_eq!(depth_data.asks.len(), 5);
    
    // Verify cumulative volumes
    let mut expected_cumulative = 0.0;
    for (i, level) in depth_data.bids.iter().enumerate() {
        expected_cumulative += level.volume;
        assert_eq!(level.cumulative_volume, expected_cumulative);
    }
    
    expected_cumulative = 0.0;
    for (i, level) in depth_data.asks.iter().enumerate() {
        expected_cumulative += level.volume;
        assert_eq!(level.cumulative_volume, expected_cumulative);
    }
}

#[test]
fn test_market_impact() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add several ask orders at different price levels
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10010.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10020.0),
        2.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10030.0),
        3.0,
        "client3".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Calculate market impact for a buy order of 2.5 BTC
    let impact = book.calculate_market_impact(OrderSide::Buy, 2.5);
    
    // This should fill 1.0 BTC at 10010.0 and 1.5 BTC at 10020.0
    assert_eq!(impact.order_size, 2.5);
    assert_eq!(impact.side, OrderSide::Buy);
    assert_eq!(impact.execution_levels.len(), 2);
    
    // Check first execution level
    assert_eq!(impact.execution_levels[0].price, price_to_internal(10010.0));
    assert_eq!(impact.execution_levels[0].executed_volume, 1.0);
    assert_eq!(impact.execution_levels[0].percentage_of_order, 40.0);
    
    // Check second execution level
    assert_eq!(impact.execution_levels[1].price, price_to_internal(10020.0));
    assert_eq!(impact.execution_levels[1].executed_volume, 1.5);
    assert_eq!(impact.execution_levels[1].percentage_of_order, 60.0);
    
    // Expected execution price: (10010.0 * 1.0 + 10020.0 * 1.5) / 2.5 = 10016.0
    assert!((impact.expected_execution_price - 10016.0).abs() < 0.01);
}

#[test]
fn test_liquidity_profile() {
    // Create a new order book
    let config = OrderBookConfig::default();
    let mut book = OrderBook::new("BTCUSD".to_string(), "EXCHANGE".to_string(), config);
    
    // Add bid orders
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(9980.0),
        1.0,
        "client1".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(9990.0),
        2.0,
        "client2".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Buy,
        price_to_internal(10000.0),
        3.0,
        "client3".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Add ask orders
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10010.0),
        1.5,
        "client4".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10020.0),
        2.5,
        "client5".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    book.add_limit_order(
        OrderSide::Sell,
        price_to_internal(10030.0),
        3.5,
        "client6".to_string(),
        TimeInForce::GTC,
    ).unwrap();
    
    // Get liquidity profile with 0.5% range
    let profile = book.get_liquidity_profile(0.5);
    
    // Mid price should be 10005.0
    assert!((profile.mid_price - 10005.0).abs() < 0.1);
    
    // Check price range (0.5% of 10005.0 = 50.025)
    assert!((profile.min_price - 9954.975).abs() < 0.1);
    assert!((profile.max_price - 10055.025).abs() < 0.1);
    
    // All bid orders should be in range (since they're all > 9954.975)
    assert_eq!(profile.bid_volume_in_range, 6.0);
    
    // All ask orders should be in range (since they're all < 10055.025)
    assert_eq!(profile.ask_volume_in_range, 7.5);
    
    // Total volume in range
    assert_eq!(profile.total_volume_in_range, 13.5);
    
    // Concentration should be 1.0 (all volume is in range)
    assert!((profile.bid_concentration - 1.0).abs() < 0.001);
    assert!((profile.ask_concentration - 1.0).abs() < 0.001);
}