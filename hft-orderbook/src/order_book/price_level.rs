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
    order_queue: OrderRingBuffer<Order>,
    /// Last update timestamp
    last_updated: u64,
    /// Map of sequence to order ID for fast lookup
    sequence_to_id: fnv::FnvHashMap<usize, OrderId>,
    /// Map of order ID to sequence for fast lookup
    id_to_sequence: fnv::FnvHashMap<OrderId, usize>,
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
    #[inline]
    pub fn new(price: Price, buffer_size: usize) -> Self {
        Self {
            price,
            total_quantity: 0.0,
            order_count: 0,
            order_queue: OrderRingBuffer::new(buffer_size),
            last_updated: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            sequence_to_id: fnv::FnvHashMap::default(),
            id_to_sequence: fnv::FnvHashMap::default(),
        }
    }

    /// Get the price of this level
    #[inline]
    pub fn price(&self) -> Price {
        self.price
    }

    /// Get the total quantity at this price level
    #[inline]
    pub fn total_quantity(&self) -> Quantity {
        self.total_quantity
    }

    /// Get the number of orders at this price level
    #[inline]
    pub fn order_count(&self) -> usize {
        self.order_count
    }

    /// Get the last update timestamp
    #[inline]
    pub fn last_updated(&self) -> u64 {
        self.last_updated
    }

    /// Add an order to this price level
    #[inline]
    pub fn add_order(&mut self, order: Order) -> OrderId {
        let id = order.id;
        let quantity = order.remaining_quantity;
        
        // Publish directly for single-producer case
        let sequence = self.order_queue.publish_direct(order);
        
        // Update mappings
        self.sequence_to_id.insert(sequence, id);
        self.id_to_sequence.insert(id, sequence);
        
        self.total_quantity += quantity;
        self.order_count += 1;
        self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        
        id
    }

    /// Remove an order by ID from this price level
    #[inline]
    pub fn remove_order_by_id(&mut self, order_id: &OrderId) -> Option<Order> {
        // Get sequence from order ID
        let sequence = *self.id_to_sequence.get(order_id)?;
        
        // Remove the order
        let order = self.order_queue.remove(sequence)?;
        
        // Update mappings
        self.sequence_to_id.remove(&sequence);
        self.id_to_sequence.remove(order_id);
        
        self.total_quantity -= order.remaining_quantity;
        self.order_count -= 1;
        self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        
        Some(order)
    }

    /// Remove an order by sequence from this price level
    #[inline]
    pub fn remove_order(&mut self, sequence: usize) -> Option<Order> {
        // Get order ID from sequence
        let order_id = *self.sequence_to_id.get(&sequence)?;
        
        // Remove the order
        let order = self.order_queue.remove(sequence)?;
        
        // Update mappings
        self.sequence_to_id.remove(&sequence);
        self.id_to_sequence.remove(&order_id);
        
        self.total_quantity -= order.remaining_quantity;
        self.order_count -= 1;
        self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        
        Some(order)
    }

    /// Get an order by ID
    #[inline]
    pub fn get_order_by_id(&self, order_id: &OrderId) -> Option<Order> {
        // Get sequence from order ID
        let sequence = *self.id_to_sequence.get(order_id)?;
        
        // Get the order
        self.order_queue.get(sequence)
    }

    /// Get an order by sequence
    #[inline]
    pub fn get_order(&self, sequence: usize) -> Option<Order> {
        self.order_queue.get(sequence)
    }

    /// Update an order by ID
    #[inline]
    pub fn update_order_by_id(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool {
        // Get sequence from order ID
        if let Some(&sequence) = self.id_to_sequence.get(order_id) {
            let result = self.order_queue.update(sequence, updater);
            
            if result {
                self.recalculate_total_quantity();
                self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            }
            
            result
        } else {
            false
        }
    }

    /// Update an order by sequence
    #[inline]
    pub fn update_order(&mut self, sequence: usize, updater: impl FnOnce(&mut Order)) -> bool {
        let result = self.order_queue.update(sequence, updater);
        
        if result {
            self.recalculate_total_quantity();
            self.last_updated = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        }
        
        result
    }

    /// Is this price level empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.order_count == 0
    }

    /// Get a serializable representation of this price level
    #[inline]
    pub fn to_data(&self) -> PriceLevelData {
        PriceLevelData {
            price: self.price,
            total_quantity: self.total_quantity,
            order_count: self.order_count,
            last_updated: self.last_updated,
        }
    }

    /// Get the next order in this price level (for matching)
    #[inline]
    pub fn next_order(&self) -> Option<(usize, Order)> {
        self.order_queue.try_next()
    }

    /// Get all orders at this price level
    #[inline]
    pub fn orders(&self) -> impl Iterator<Item = Order> + '_ {
        self.order_queue.iter()
    }

    // Recalculate the total quantity based on all orders
    #[inline]
    fn recalculate_total_quantity(&mut self) {
        self.total_quantity = self.order_queue.iter()
            .map(|order| order.remaining_quantity)
            .sum();
    }
}

impl PartialEq for PriceLevel {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl Eq for PriceLevel {}

impl PartialOrd for PriceLevel {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriceLevel {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.price.cmp(&other.price)
    }
}