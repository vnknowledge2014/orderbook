//! Order book implementation for high-frequency trading.

mod order;
mod price_level;
mod side;
pub mod book;  // Make book module public
pub mod market_depth;

pub use order::{Order, OrderId, OrderType, OrderSide, OrderStatus, TimeInForce};
pub use price_level::PriceLevel;
pub use side::{Side, BidSide, AskSide};
pub use book::OrderBook;
pub use book::OrderBookConfig;  // Export OrderBookConfig directly

/// Price representation - using a fixed-point representation for performance
pub type Price = i64;

/// Quantity representation
pub type Quantity = f64;

/// Timestamp in nanoseconds
pub type Timestamp = u64;

/// Client identifier
pub type ClientId = String;

/// Constants for price conversion (to avoid floating point errors)
pub const PRICE_MULTIPLIER: i64 = 100_000; // 5 decimal places

/// Convert a floating point price to the internal fixed-point representation
#[inline]
pub fn price_to_internal(price: f64) -> Price {
    (price * PRICE_MULTIPLIER as f64) as i64
}

/// Convert the internal fixed-point representation to a floating point price
#[inline]
pub fn price_from_internal(price: Price) -> f64 {
    price as f64 / PRICE_MULTIPLIER as f64
}
