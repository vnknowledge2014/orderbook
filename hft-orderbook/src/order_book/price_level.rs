//! Price level in the order book.

use serde::{Serialize, Deserialize};
use crate::order_book::{Order, OrderId, Price, Quantity};
use crate::queue::OrderRingBuffer;

/// Price level in the order book
#[derive(Debug)]
pub struct PriceLevel {
    /// Price of this level
    price: Price,
    /// Total quantity at this price level
    total_quantity: Quantity,
    /// Number of orders at this price level
    order_count: usize,
    /// Queue of orders at this price level
    order_queue: OrderRingBuffer,
    /// Last update timestamp
    last_updated: u64,
}

/// Price level data for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevelData {
    /// Price of this level
    pub price: Price,
    /// Total quantity at this price level
    pub total_quantity: Quantity,
    /// Number of orders at this price level
    pub order_count: usize,
    /// Last update timestamp
    pub last_updated: u64,
}

impl PriceLevel {
    /// Create a new price level
    pub fn new(price: Price, buffer_size: usize) -> Self {
        Self {
            price,
            total_quantity: 0.0,
            order_count: 0,
            order_queue: OrderRingBuffer::new(buffer_size),
            last_updated: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
        }
    }

    /// Get the price of this level
    pub fn price(&self) -> Price {
        self.price
    }

    /// Get the total quantity at this price level
    pub fn total_quantity(&self) -> Quantity {
        self.total_quantity
    }

    /// Get the number of orders at this price level
    pub fn order_count(&self) -> usize {
        self.order_count
    }

    /// Get the last update timestamp
    pub fn last_updated(&self) -> u64 {
        self.last_updated
    }

    /// Add an order to this price level
    pub fn add_order(&mut self, order: Order) -> OrderId {
        let id = order.id;
        let quantity = order.remaining_quantity;
        
        self.order_queue.publish(order);
        
        self.total_quantity += quantity;
        self.order_count += 1;
        self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        
        id
    }

    /// Remove an order from this price level
    pub fn remove_order(&mut self, position: usize) -> Option<Order> {
        let order = self.order_queue.remove(position)?;
        
        self.total_quantity -= order.remaining_quantity;
        self.order_count -= 1;
        self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        
        Some(order)
    }

    /// Get an order by position
    pub fn get_order(&self, position: usize) -> Option<Order> {
        self.order_queue.get(position)
    }

    /// Update an order in this price level
    pub fn update_order(&mut self, position: usize, updater: impl FnOnce(&mut Order)) -> bool {
        let result = self.order_queue.update(position, updater);
        
        if result {
            self.recalculate_total_quantity();
            self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        }
        
        result
    }

    /// Is this price level empty
    pub fn is_empty(&self) -> bool {
        self.order_count == 0
    }

    /// Get a serializable representation of this price level
    pub fn to_data(&self) -> PriceLevelData {
        PriceLevelData {
            price: self.price,
            total_quantity: self.total_quantity,
            order_count: self.order_count,
            last_updated: self.last_updated,
        }
    }

    // Recalculate the total quantity based on all orders
    fn recalculate_total_quantity(&mut self) {
        self.total_quantity = self.order_queue.iter()
            .map(|order| order.remaining_quantity)
            .sum();
    }
}

impl PartialEq for PriceLevel {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl Eq for PriceLevel {}

impl PartialOrd for PriceLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriceLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.price.cmp(&other.price)
    }
}
