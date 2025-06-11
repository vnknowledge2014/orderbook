//! # HFT Order Book
//! 
//! A high-performance order book implementation for high-frequency trading (HFT) systems.
//! This implementation uses a Red-Black Tree for price levels and LMAX Disruptor pattern
//! with ring buffers for the order queue.

pub mod order_book;
pub mod matching_engine;
pub mod queue;
pub mod simulation;
pub mod util;

pub use order_book::{OrderBook, Order, PriceLevel, Side, OrderId, Price, Quantity};
pub use matching_engine::MatchingEngine;
