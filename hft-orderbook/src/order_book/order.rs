//! Order representation for the order book.

use std::cmp::Ordering;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::order_book::{Price, Quantity, Timestamp, ClientId};

/// Unique identifier for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

impl OrderId {
    /// Create a new random order ID
    pub fn new() -> Self {
        OrderId(Uuid::new_v4())
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    /// Buy order (bid)
    Buy,
    /// Sell order (ask)
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order with a specific price
    Limit,
    /// Market order executed at best available price
    Market,
    /// Stop order activated when price reaches a specific level
    Stop,
    /// Stop limit order
    StopLimit,
}

/// Time in force for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till canceled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
    /// Good till date
    GTD(Timestamp),
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is active and in the book
    Active,
    /// Order has been partially filled
    PartiallyFilled,
    /// Order has been completely filled
    Filled,
    /// Order has been canceled
    Canceled,
    /// Order has been rejected
    Rejected,
    /// Order is stopped and waiting for activation
    Stopped,
}

/// Execution of an order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Execution {
    /// Timestamp of the execution
    pub timestamp: Timestamp,
    /// Quantity executed
    pub quantity: Quantity,
    /// Execution price
    pub price: Price,
    /// Counterparty order ID
    pub counterparty_id: OrderId,
}

/// Order in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order ID
    pub id: OrderId,
    /// Order price
    pub price: Price,
    /// Original order quantity
    pub original_quantity: Quantity,
    /// Remaining quantity
    pub remaining_quantity: Quantity,
    /// Order side (buy or sell)
    pub side: OrderSide,
    /// Timestamp of the order
    pub timestamp: Timestamp,
    /// Client ID
    pub client_id: ClientId,
    /// Order status
    pub status: OrderStatus,
    /// Order type
    pub order_type: OrderType,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Executed quantity
    pub executed_quantity: Quantity,
    /// Average execution price
    pub average_execution_price: Option<Price>,
    /// List of executions
    pub executions: Vec<Execution>,
    /// Index in the price level queue
    pub queue_position: Option<usize>,
}

impl Order {
    /// Create a new limit order
    pub fn new_limit(
        side: OrderSide, 
        price: Price, 
        quantity: Quantity, 
        client_id: ClientId,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            id: OrderId::new(),
            price,
            original_quantity: quantity,
            remaining_quantity: quantity,
            side,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            client_id,
            status: OrderStatus::Active,
            order_type: OrderType::Limit,
            time_in_force,
            executed_quantity: 0.0,
            average_execution_price: None,
            executions: Vec::new(),
            queue_position: None,
        }
    }

    /// Create a new market order
    pub fn new_market(
        side: OrderSide, 
        quantity: Quantity, 
        client_id: ClientId,
    ) -> Self {
        Self {
            id: OrderId::new(),
            price: 0, // Market orders don't have a price
            original_quantity: quantity,
            remaining_quantity: quantity,
            side,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            client_id,
            status: OrderStatus::Active,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::IOC, // Market orders are IOC by default
            executed_quantity: 0.0,
            average_execution_price: None,
            executions: Vec::new(),
            queue_position: None,
        }
    }

    /// Add an execution to the order
    pub fn add_execution(&mut self, execution: Execution) {
        self.executed_quantity += execution.quantity;
        self.remaining_quantity -= execution.quantity;
        
        // Update average execution price
        let total_executed_value = if let Some(avg_price) = self.average_execution_price {
            (self.executed_quantity - execution.quantity) * avg_price as f64
        } else {
            0.0
        };
        
        let new_execution_value = execution.quantity * execution.price as f64;
        let new_average_price = ((total_executed_value + new_execution_value) / self.executed_quantity) as i64;
        
        self.average_execution_price = Some(new_average_price);
        self.executions.push(execution);
        
        // Update status
        if self.remaining_quantity <= 0.0 {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }
    }

    /// Cancel the order
    pub fn cancel(&mut self) {
        if self.status != OrderStatus::Filled {
            self.status = OrderStatus::Canceled;
        }
    }

    /// Is the order active
    pub fn is_active(&self) -> bool {
        matches!(self.status, OrderStatus::Active | OrderStatus::PartiallyFilled)
    }
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Order {}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        // Orders are ordered by time priority (FIFO)
        self.timestamp.cmp(&other.timestamp)
    }
}
