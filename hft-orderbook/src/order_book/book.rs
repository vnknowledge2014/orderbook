//! Main order book implementation.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::order_book::{
    Order, OrderId, Price, Quantity, Timestamp, ClientId, 
    PriceLevel, Side, BidSide, AskSide, 
    OrderSide, OrderStatus, OrderType, TimeInForce
};
use crate::order_book::price_level::PriceLevelData;

/// Order book for a single instrument
#[derive(Debug)]
pub struct OrderBook {
    /// Symbol of the instrument
    symbol: String,
    /// Exchange ID
    exchange_id: String,
    /// Bid side of the order book
    bid_side: BidSide,
    /// Ask side of the order book
    ask_side: AskSide,
    /// Map of all orders in the book
    order_map: HashMap<OrderId, (OrderSide, Price)>,
    /// Last update ID
    last_update_id: u64,
    /// Timestamp of the last update
    timestamp: Timestamp,
    /// Configuration
    config: OrderBookConfig,
}

/// Order book configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookConfig {
    /// Tick size (minimum price increment)
    pub tick_size: f64,
    /// Lot size (minimum quantity increment)
    pub lot_size: f64,
    /// Minimum order size
    pub min_order_size: f64,
    /// Maximum order size
    pub max_order_size: f64,
    /// Maximum number of price levels
    pub max_levels: usize,
    /// Maximum number of orders per price level
    pub max_orders_per_level: usize,
    /// Maximum total orders in the book
    pub max_total_orders: usize,
    /// Buffer size for each price level
    pub buffer_size: usize,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            tick_size: 0.01,
            lot_size: 0.0001,
            min_order_size: 0.001,
            max_order_size: 1000.0,
            max_levels: 1000,
            max_orders_per_level: 10000,
            max_total_orders: 1000000,
            buffer_size: 1024,
        }
    }
}

/// Order book data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    /// Symbol of the instrument
    pub symbol: String,
    /// Exchange ID
    pub exchange_id: String,
    /// Timestamp of the last update
    pub timestamp: Timestamp,
    /// Last update ID
    pub last_update_id: u64,
    /// Bid side of the order book
    pub bid_side: SideData,
    /// Ask side of the order book
    pub ask_side: SideData,
    /// Statistics
    pub statistics: OrderBookStatistics,
    /// Market data snapshot
    pub market_data: MarketDataSnapshot,
    /// Configuration
    pub config: OrderBookConfig,
}

/// Order book statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookStatistics {
    /// Spread in price units
    pub spread: f64,
    /// Spread in basis points
    pub spread_bps: f64,
    /// Mid price
    pub mid_price: f64,
    /// Last trade price
    pub last_trade_price: Option<f64>,
    /// Last trade quantity
    pub last_trade_quantity: Option<f64>,
    /// Last trade timestamp
    pub last_trade_timestamp: Option<Timestamp>,
    /// 24-hour volume
    pub volume_24h: f64,
    /// 24-hour price change (percentage)
    pub price_change_24h: f64,
    /// Update frequency (updates per second)
    pub update_frequency: u64,
    /// Order depth (total price levels)
    pub order_depth: usize,
}

/// Side data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideData {
    /// Best price on this side
    pub best_price: Option<Price>,
    /// Total volume on this side
    pub total_volume: Quantity,
    /// Depth of this side
    pub depth: usize,
    /// Price levels
    pub price_levels: Vec<(Price, PriceLevelData)>,
    /// Statistics
    pub statistics: SideStatistics,
}

/// Side statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideStatistics {
    /// Average order size
    pub average_order_size: Quantity,
    /// Number of orders
    pub order_count: usize,
    /// Depth in pips (difference between best and worst price)
    pub depth_in_pips: i64,
}

/// Market data snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: Timestamp,
    /// Last update ID
    pub last_update_id: u64,
    /// Bid levels (price, quantity)
    pub bids: Vec<(Price, Quantity)>,
    /// Ask levels (price, quantity)
    pub asks: Vec<(Price, Quantity)>,
}

impl OrderBook {
    /// Create a new order book
    pub fn new(symbol: String, exchange_id: String, config: OrderBookConfig) -> Self {
        Self {
            symbol,
            exchange_id,
            bid_side: BidSide::new(config.buffer_size),
            ask_side: AskSide::new(config.buffer_size),
            order_map: HashMap::new(),
            last_update_id: 0,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            config,
        }
    }
    
    /// Add a limit order to the book
    pub fn add_limit_order(
        &mut self,
        side: OrderSide,
        price: Price,
        quantity: Quantity,
        client_id: ClientId,
        time_in_force: TimeInForce,
    ) -> Result<OrderId, OrderBookError> {
        // Validate order
        self.validate_order(price, quantity)?;
        
        // Create order
        let order = Order::new_limit(side, price, quantity, client_id, time_in_force);
        let order_id = order.id;
        
        // Add order to the appropriate side
        match side {
            OrderSide::Buy => {
                let id = self.bid_side.add_order(order);
                self.order_map.insert(id, (OrderSide::Buy, price));
            }
            OrderSide::Sell => {
                let id = self.ask_side.add_order(order);
                self.order_map.insert(id, (OrderSide::Sell, price));
            }
        }
        
        // Update timestamp and ID
        self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        self.last_update_id += 1;
        
        Ok(order_id)
    }
    
    /// Cancel an order
    pub fn cancel_order(&mut self, order_id: &OrderId) -> Result<Order, OrderBookError> {
        // Get side and price from order map
        let (side, _) = self.order_map.get(order_id)
            .ok_or(OrderBookError::OrderNotFound)?;
        
        // Remove order from the appropriate side
        let order = match side {
            OrderSide::Buy => self.bid_side.remove_order(order_id),
            OrderSide::Sell => self.ask_side.remove_order(order_id),
        }.ok_or(OrderBookError::OrderNotFound)?;
        
        // Remove from order map
        self.order_map.remove(order_id);
        
        // Update timestamp and ID
        self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        self.last_update_id += 1;
        
        Ok(order)
    }
    
    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Price> {
        self.bid_side.best_price()
    }
    
    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Price> {
        self.ask_side.best_price()
    }
    
    /// Get the spread
    pub fn spread(&self) -> Option<Price> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }
    
    /// Get the mid price
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => {
                let ask_f = ask as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                let bid_f = bid as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                Some((ask_f + bid_f) / 2.0)
            }
            _ => None,
        }
    }
    
    /// Get an order by ID
    pub fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Get side and price from order map
        let (side, _) = self.order_map.get(order_id)?;
        
        // Get order from the appropriate side
        match side {
            OrderSide::Buy => self.bid_side.get_order(order_id),
            OrderSide::Sell => self.ask_side.get_order(order_id),
        }
    }
    
    /// Get the top N bid levels
    pub fn top_bids(&self, n: usize) -> Vec<(Price, Quantity)> {
        self.bid_side.price_levels().into_iter()
            .take(n)
            .map(|(price, level)| (price, level.total_quantity))
            .collect()
    }
    
    /// Get the top N ask levels
    pub fn top_asks(&self, n: usize) -> Vec<(Price, Quantity)> {
        self.ask_side.price_levels().into_iter()
            .take(n)
            .map(|(price, level)| (price, level.total_quantity))
            .collect()
    }
    
    /// Get a serializable representation of the order book
    pub fn to_data(&self) -> OrderBookData {
        // Calculate statistics
        let spread = match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => {
                let ask_f = ask as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                let bid_f = bid as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                ask_f - bid_f
            }
            _ => 0.0,
        };
        
        let mid_price = self.mid_price().unwrap_or(0.0);
        
        let spread_bps = if mid_price > 0.0 {
            (spread / mid_price) * 10000.0
        } else {
            0.0
        };
        
        // TODO: Implement these statistics properly
        let last_trade_price = None;
        let last_trade_quantity = None;
        let last_trade_timestamp = None;
        let volume_24h = 0.0;
        let price_change_24h = 0.0;
        let update_frequency = 0;
        
        // Create market data snapshot
        let market_data = MarketDataSnapshot {
            timestamp: self.timestamp,
            last_update_id: self.last_update_id,
            bids: self.top_bids(10),
            asks: self.top_asks(10),
        };
        
        let bid_side_data = SideData { 
            best_price: self.best_bid(),
            total_volume: self.bid_side.total_volume(),
            depth: self.bid_side.depth(),
            price_levels: self.bid_side.price_levels(),
            statistics: SideStatistics {
                average_order_size: if self.bid_side.depth() > 0 {
                    self.bid_side.total_volume() / self.bid_side.depth() as f64
                } else {
                    0.0
                },
                order_count: self.bid_side.price_levels().iter()
                    .map(|(_, level)| level.order_count)
                    .sum(),
                depth_in_pips: 0, // TODO: Implement this properly
            },
        };
        
        let ask_side_data = SideData {
            best_price: self.best_ask(),
            total_volume: self.ask_side.total_volume(),
            depth: self.ask_side.depth(),
            price_levels: self.ask_side.price_levels(),
            statistics: SideStatistics {
                average_order_size: if self.ask_side.depth() > 0 {
                    self.ask_side.total_volume() / self.ask_side.depth() as f64
                } else {
                    0.0
                },
                order_count: self.ask_side.price_levels().iter()
                    .map(|(_, level)| level.order_count)
                    .sum(),
                depth_in_pips: 0, // TODO: Implement this properly
            },
        };
        
        OrderBookData {
            symbol: self.symbol.clone(),
            exchange_id: self.exchange_id.clone(),
            timestamp: self.timestamp,
            last_update_id: self.last_update_id,
            bid_side: bid_side_data,
            ask_side: ask_side_data,
            statistics: OrderBookStatistics {
                spread,
                spread_bps,
                mid_price,
                last_trade_price,
                last_trade_quantity,
                last_trade_timestamp,
                volume_24h,
                price_change_24h,
                update_frequency,
                order_depth: self.bid_side.depth() + self.ask_side.depth(),
            },
            market_data,
            config: self.config.clone(),
        }
    }
    
    // Validate an order
    fn validate_order(&self, _price: Price, quantity: Quantity) -> Result<(), OrderBookError> {
        // Check if the order size is valid
        if quantity < self.config.min_order_size {
            return Err(OrderBookError::InvalidOrderSize("Order size too small".to_string()));
        }
        
        if quantity > self.config.max_order_size {
            return Err(OrderBookError::InvalidOrderSize("Order size too large".to_string()));
        }
        
        // Check if the order book is full
        if self.order_map.len() >= self.config.max_total_orders {
            return Err(OrderBookError::OrderBookFull);
        }
        
        Ok(())
    }
}

/// Order book error
#[derive(Debug, thiserror::Error)]
pub enum OrderBookError {
    /// Invalid order size
    #[error("Invalid order size: {0}")]
    InvalidOrderSize(String),
    
    /// Order not found
    #[error("Order not found")]
    OrderNotFound,
    
    /// Order book full
    #[error("Order book full")]
    OrderBookFull,
    
    /// Price level full
    #[error("Price level full")]
    PriceLevelFull,
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}
