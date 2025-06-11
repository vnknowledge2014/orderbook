//! Implementations of bid and ask sides of the order book.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use crate::order_book::{Order, OrderId, Price, Quantity, PriceLevel, OrderSide};
use crate::order_book::price_level::PriceLevelData;

/// Generic side of the order book (either bid or ask)
pub trait Side {
    /// Add an order to this side
    fn add_order(&mut self, order: Order) -> OrderId;
    
    /// Remove an order from this side
    fn remove_order(&mut self, order_id: &OrderId) -> Option<Order>;
    
    /// Get the best price on this side
    fn best_price(&self) -> Option<Price>;
    
    /// Get the total volume on this side
    fn total_volume(&self) -> Quantity;
    
    /// Get the depth of this side (number of price levels)
    fn depth(&self) -> usize;
    
    /// Get an order by ID
    fn get_order(&self, order_id: &OrderId) -> Option<Order>;
    
    /// Update an order
    fn update_order(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool;
    
    /// Get the price levels sorted by price
    fn price_levels(&self) -> Vec<(Price, PriceLevelData)>;
    
    /// Get a specific price level
    fn get_price_level(&self, price: Price) -> Option<PriceLevelData>;
}

/// Data structure for serializing a side of the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideData {
    /// Best price on this side
    pub best_price: Option<Price>,
    /// Total volume on this side
    pub total_volume: Quantity,
    /// Number of price levels
    pub depth: usize,
    /// Price levels
    pub price_levels: Vec<(Price, PriceLevelData)>,
    /// Statistics
    pub statistics: SideStatistics,
}

/// Statistics for a side of the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideStatistics {
    /// Average order size
    pub average_order_size: Quantity,
    /// Number of orders
    pub order_count: usize,
    /// Depth in pips (difference between best and worst price)
    pub depth_in_pips: i64,
}

/// Common implementation for both sides of the order book
#[derive(Debug)]
pub struct BookSide {
    /// Price levels sorted by price
    price_levels: BTreeMap<Price, PriceLevel>,
    /// Map from order ID to (price, position) for fast lookup
    order_map: HashMap<OrderId, (Price, usize)>,
    /// Total volume on this side
    total_volume: Quantity,
    /// Side type (bid or ask)
    side_type: OrderSide,
    /// Buffer size for each price level
    buffer_size: usize,
}

impl BookSide {
    /// Create a new book side
    pub fn new(side_type: OrderSide, buffer_size: usize) -> Self {
        Self {
            price_levels: BTreeMap::new(),
            order_map: HashMap::new(),
            total_volume: 0.0,
            side_type,
            buffer_size,
        }
    }
    
    /// Get a serializable representation of this side
    pub fn to_data(&self) -> SideData {
        // Calculate statistics
        let order_count = self.order_map.len();
        let average_order_size = if order_count > 0 {
            self.total_volume / order_count as f64
        } else {
            0.0
        };
        
        let depth_in_pips = if let (Some(best), Some(worst)) = (self.get_best_price(), self.get_worst_price()) {
            (worst - best).abs()
        } else {
            0
        };
        
        SideData {
            best_price: self.get_best_price(),
            total_volume: self.total_volume,
            depth: self.price_levels.len(),
            price_levels: self.get_price_levels(),
            statistics: SideStatistics {
                average_order_size,
                order_count,
                depth_in_pips,
            },
        }
    }
    
    /// Get the best price on this side
    fn get_best_price(&self) -> Option<Price> {
        match self.side_type {
            OrderSide::Buy => self.price_levels.keys().rev().next().copied(),
            OrderSide::Sell => self.price_levels.keys().next().copied(),
        }
    }
    
    /// Get the worst price on this side
    fn get_worst_price(&self) -> Option<Price> {
        match self.side_type {
            OrderSide::Buy => self.price_levels.keys().next().copied(),
            OrderSide::Sell => self.price_levels.keys().rev().next().copied(),
        }
    }
    
    /// Get the price levels sorted by price
    fn get_price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        match self.side_type {
            OrderSide::Buy => self.price_levels.iter()
                .rev() // Reverse to get descending order for bids
                .map(|(price, level)| (*price, level.to_data()))
                .collect(),
            OrderSide::Sell => self.price_levels.iter()
                .map(|(price, level)| (*price, level.to_data()))
                .collect(),
        }
    }
}

/// Bid side of the order book (buy orders)
#[derive(Debug)]
pub struct BidSide(BookSide);

impl BidSide {
    /// Create a new bid side
    pub fn new(buffer_size: usize) -> Self {
        Self(BookSide::new(OrderSide::Buy, buffer_size))
    }
}

impl Side for BidSide {
    fn add_order(&mut self, order: Order) -> OrderId {
        assert_eq!(order.side, OrderSide::Buy, "Order side must be Buy for BidSide");
        
        let id = order.id;
        let price = order.price;
        let quantity = order.remaining_quantity;
        
        // Get or create price level
        let price_level = self.0.price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel::new(price, self.0.buffer_size));
        
        // Add order to price level
        let position = price_level.order_count();
        let order_id = price_level.add_order(order);
        
        // Update order map
        self.0.order_map.insert(order_id, (price, position));
        
        // Update total volume
        self.0.total_volume += quantity;
        
        id
    }
    
    fn remove_order(&mut self, order_id: &OrderId) -> Option<Order> {
        // Get price and position from order map
        let (price, position) = self.0.order_map.remove(order_id)?;
        
        // Get price level
        let price_level = self.0.price_levels.get_mut(&price)?;
        
        // Remove order from price level
        let order = price_level.remove_order(position)?;
        
        // Update total volume
        self.0.total_volume -= order.remaining_quantity;
        
        // Remove price level if empty
        if price_level.is_empty() {
            self.0.price_levels.remove(&price);
        }
        
        Some(order)
    }
    
    fn best_price(&self) -> Option<Price> {
        // For bid side, the best price is the highest (last key in descending order)
        self.0.get_best_price()
    }
    
    fn total_volume(&self) -> Quantity {
        self.0.total_volume
    }
    
    fn depth(&self) -> usize {
        self.0.price_levels.len()
    }
    
    fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Get price and position from order map
        let (price, position) = self.0.order_map.get(order_id)?;
        
        // Get price level
        let price_level = self.0.price_levels.get(price)?;
        
        // Get order from price level
        price_level.get_order(*position)
    }
    
    fn update_order(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool {
        // Get price and position from order map
        if let Some((price, position)) = self.0.order_map.get(order_id) {
            // Get price level
            if let Some(price_level) = self.0.price_levels.get_mut(price) {
                // Update order in price level
                let result = price_level.update_order(*position, updater);
                
                // Recalculate total volume
                if result {
                    self.0.total_volume = self.0.price_levels.values()
                        .map(|level| level.total_quantity())
                        .sum();
                }
                
                return result;
            }
        }
        
        false
    }
    
    fn price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        // For bid side, price levels are sorted in descending order
        self.0.get_price_levels()
    }
    
    fn get_price_level(&self, price: Price) -> Option<PriceLevelData> {
        self.0.price_levels.get(&price).map(|level| level.to_data())
    }
}

/// Ask side of the order book (sell orders)
#[derive(Debug)]
pub struct AskSide(BookSide);

impl AskSide {
    /// Create a new ask side
    pub fn new(buffer_size: usize) -> Self {
        Self(BookSide::new(OrderSide::Sell, buffer_size))
    }
}

impl Side for AskSide {
    fn add_order(&mut self, order: Order) -> OrderId {
        assert_eq!(order.side, OrderSide::Sell, "Order side must be Sell for AskSide");
        
        let id = order.id;
        let price = order.price;
        let quantity = order.remaining_quantity;
        
        // Get or create price level
        let price_level = self.0.price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel::new(price, self.0.buffer_size));
        
        // Add order to price level
        let position = price_level.order_count();
        let order_id = price_level.add_order(order);
        
        // Update order map
        self.0.order_map.insert(order_id, (price, position));
        
        // Update total volume
        self.0.total_volume += quantity;
        
        id
    }
    
    fn remove_order(&mut self, order_id: &OrderId) -> Option<Order> {
        // Get price and position from order map
        let (price, position) = self.0.order_map.remove(order_id)?;
        
        // Get price level
        let price_level = self.0.price_levels.get_mut(&price)?;
        
        // Remove order from price level
        let order = price_level.remove_order(position)?;
        
        // Update total volume
        self.0.total_volume -= order.remaining_quantity;
        
        // Remove price level if empty
        if price_level.is_empty() {
            self.0.price_levels.remove(&price);
        }
        
        Some(order)
    }
    
    fn best_price(&self) -> Option<Price> {
        // For ask side, the best price is the lowest (first key in ascending order)
        self.0.get_best_price()
    }
    
    fn total_volume(&self) -> Quantity {
        self.0.total_volume
    }
    
    fn depth(&self) -> usize {
        self.0.price_levels.len()
    }
    
    fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Get price and position from order map
        let (price, position) = self.0.order_map.get(order_id)?;
        
        // Get price level
        let price_level = self.0.price_levels.get(price)?;
        
        // Get order from price level
        price_level.get_order(*position)
    }
    
    fn update_order(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool {
        // Get price and position from order map
        if let Some((price, position)) = self.0.order_map.get(order_id) {
            // Get price level
            if let Some(price_level) = self.0.price_levels.get_mut(price) {
                // Update order in price level
                let result = price_level.update_order(*position, updater);
                
                // Recalculate total volume
                if result {
                    self.0.total_volume = self.0.price_levels.values()
                        .map(|level| level.total_quantity())
                        .sum();
                }
                
                return result;
            }
        }
        
        false
    }
    
    fn price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        // For ask side, price levels are sorted in ascending order
        self.0.get_price_levels()
    }
    
    fn get_price_level(&self, price: Price) -> Option<PriceLevelData> {
        self.0.price_levels.get(&price).map(|level| level.to_data())
    }
}
