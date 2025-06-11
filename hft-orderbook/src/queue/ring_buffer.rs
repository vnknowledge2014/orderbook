//! Ring buffer implementation inspired by LMAX Disruptor pattern.

use std::sync::Arc;
use parking_lot::{Mutex, RwLock};
use crate::order_book::{Order, OrderId};

/// Ring buffer for orders
#[derive(Debug)]
pub struct OrderRingBuffer {
    /// Buffer for orders
    buffer: Vec<Option<Order>>,
    /// Current position in the buffer
    position: usize,
    /// Capacity of the buffer
    capacity: usize,
}

impl OrderRingBuffer {
    /// Create a new order ring buffer
    pub fn new(capacity: usize) -> Self {
        // Create a buffer with the given capacity
        let buffer = vec![None; capacity];
        
        Self {
            buffer,
            position: 0,
            capacity,
        }
    }
    
    /// Publish an order to the buffer
    pub fn publish(&mut self, order: Order) {
        let index = self.position % self.capacity;
        
        // Store the order in the buffer
        self.buffer[index] = Some(order);
        
        // Update the position
        self.position += 1;
    }
    
    /// Remove an order from the buffer
    pub fn remove(&mut self, position: usize) -> Option<Order> {
        if position >= self.position {
            return None;
        }
        
        let index = position % self.capacity;
        
        // Get the order from the buffer
        let order = self.buffer[index].take();
        
        order
    }
    
    /// Get an order from the buffer
    pub fn get(&self, position: usize) -> Option<Order> {
        if position >= self.position {
            return None;
        }
        
        let index = position % self.capacity;
        
        // Get the order from the buffer
        self.buffer[index].clone()
    }
    
    /// Update an order in the buffer
    pub fn update(&mut self, position: usize, updater: impl FnOnce(&mut Order)) -> bool {
        if position >= self.position {
            return false;
        }
        
        let index = position % self.capacity;
        
        // Get the order from the buffer
        if let Some(ref mut order) = self.buffer[index] {
            updater(order);
            return true;
        }
        
        false
    }
    
    /// Get an iterator over the orders in the buffer
    pub fn iter(&self) -> impl Iterator<Item = Order> + '_ {
        (0..self.position)
            .filter_map(move |pos| self.get(pos))
    }
}
