//! Implementations of bid and ask sides of the order book.

use crate::order_book::price_level::PriceLevelData;
use crate::order_book::{Order, OrderId, OrderSide, Price, PriceLevel, Quantity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

    /// Get the next order at the best price (for matching)
    fn next_best_order(&mut self) -> Option<Order>;
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
    /// Map from order ID to price for fast lookup
    order_map: fnv::FnvHashMap<OrderId, Price>,
    /// Total volume on this side
    total_volume: Quantity,
    /// Side type (bid or ask)
    side_type: OrderSide,
    /// Buffer size for each price level
    buffer_size: usize,
    /// Cache of best price for O(1) lookup
    best_price_cache: Option<Price>,
}

impl BookSide {
    /// Create a new book side
    #[inline]
    pub fn new(side_type: OrderSide, buffer_size: usize) -> Self {
        Self {
            price_levels: BTreeMap::new(),
            order_map: fnv::FnvHashMap::default(),
            total_volume: 0.0,
            side_type,
            buffer_size,
            best_price_cache: None,
        }
    }

    /// Get the best price on this side
    #[inline]
    fn get_best_price(&self) -> Option<Price> {
        // Return cached value if available
        if let Some(price) = self.best_price_cache {
            return Some(price);
        }

        match self.side_type {
            OrderSide::Buy => self.price_levels.keys().rev().next().copied(),
            OrderSide::Sell => self.price_levels.keys().next().copied(),
        }
    }

    /// Get the worst price on this side
    #[inline]
    fn get_worst_price(&self) -> Option<Price> {
        match self.side_type {
            OrderSide::Buy => self.price_levels.keys().next().copied(),
            OrderSide::Sell => self.price_levels.keys().rev().next().copied(),
        }
    }

    /// Get the price levels sorted by price
    #[inline]
    fn get_price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        match self.side_type {
            OrderSide::Buy => self
                .price_levels
                .iter()
                .rev() // Reverse to get descending order for bids
                .map(|(price, level)| (*price, level.to_data()))
                .collect(),
            OrderSide::Sell => self
                .price_levels
                .iter()
                .map(|(price, level)| (*price, level.to_data()))
                .collect(),
        }
    }

    /// Update the best price cache
    #[inline]
    fn update_best_price_cache(&mut self) {
        self.best_price_cache = match self.side_type {
            OrderSide::Buy => self.price_levels.keys().rev().next().copied(),
            OrderSide::Sell => self.price_levels.keys().next().copied(),
        };
    }

    /// Lấy giá tệ nhất trên side này
    #[inline]
    pub fn get_worst_price(&self) -> Option<Price> {
        match self.side_type {
            OrderSide::Buy => self.price_levels.keys().next().copied(),
            OrderSide::Sell => self.price_levels.keys().rev().next().copied(),
        }
    }

    /// Lấy số lượng mức giá trong một khoảng giá
    #[inline]
    pub fn count_levels_in_range(&self, min_price: Price, max_price: Price) -> usize {
        match self.side_type {
            OrderSide::Buy => self
                .price_levels
                .range((
                    std::ops::Bound::Included(min_price),
                    std::ops::Bound::Included(max_price),
                ))
                .count(),
            OrderSide::Sell => self
                .price_levels
                .range((
                    std::ops::Bound::Included(min_price),
                    std::ops::Bound::Included(max_price),
                ))
                .count(),
        }
    }

    /// Lấy tổng khối lượng trong một khoảng giá
    #[inline]
    pub fn total_volume_in_range(&self, min_price: Price, max_price: Price) -> Quantity {
        match self.side_type {
            OrderSide::Buy => self
                .price_levels
                .range((
                    std::ops::Bound::Included(min_price),
                    std::ops::Bound::Included(max_price),
                ))
                .map(|(_, level)| level.total_quantity())
                .sum(),
            OrderSide::Sell => self
                .price_levels
                .range((
                    std::ops::Bound::Included(min_price),
                    std::ops::Bound::Included(max_price),
                ))
                .map(|(_, level)| level.total_quantity())
                .sum(),
        }
    }

    /// Lấy phân phối khối lượng theo mức giá
    pub fn volume_distribution(&self, num_buckets: usize) -> Vec<(Price, Quantity)> {
        if self.price_levels.is_empty() || num_buckets == 0 {
            return Vec::new();
        }

        // Lấy giá tốt nhất và tệ nhất
        let best_price = self.get_best_price();
        let worst_price = self.get_worst_price();

        if best_price.is_none() || worst_price.is_none() {
            return Vec::new();
        }

        let best = best_price.unwrap();
        let worst = worst_price.unwrap();

        // Tính kích thước mỗi bucket
        let price_range = (best as i128 - worst as i128).abs() as u64;

        if price_range == 0 {
            // Chỉ có một mức giá
            return vec![(best, self.total_volume())];
        }

        let bucket_size = (price_range / num_buckets as u64).max(1) as i64;

        // Khởi tạo buckets
        let mut buckets = vec![(0, 0.0); num_buckets];

        // Phân phối khối lượng vào các buckets
        for (price, level) in self.price_levels.iter() {
            let relative_price = match self.side_type {
                OrderSide::Buy => best - *price,
                OrderSide::Sell => *price - best,
            };

            let bucket_index = (relative_price / bucket_size) as usize;

            if bucket_index < num_buckets {
                let bucket_price = match self.side_type {
                    OrderSide::Buy => best - bucket_index as i64 * bucket_size,
                    OrderSide::Sell => best + bucket_index as i64 * bucket_size,
                };

                buckets[bucket_index].0 = bucket_price;
                buckets[bucket_index].1 += level.total_quantity();
            }
        }

        buckets
    }

    /// Tính độ dốc (slope) của order book
    pub fn calculate_slope(&self) -> f64 {
        if self.price_levels.is_empty() {
            return 0.0;
        }

        let best_price = match self.get_best_price() {
            Some(price) => price as f64 / crate::order_book::PRICE_MULTIPLIER as f64,
            None => return 0.0,
        };

        let worst_price = match self.get_worst_price() {
            Some(price) => price as f64 / crate::order_book::PRICE_MULTIPLIER as f64,
            None => return 0.0,
        };

        // Nếu chỉ có một mức giá
        if (best_price - worst_price).abs() < 1e-6 {
            return 0.0;
        }

        // Tính tổng khối lượng
        let total_volume = self.total_volume();

        // Tính độ dốc (volume / price range)
        match self.side_type {
            OrderSide::Buy => total_volume / (best_price - worst_price).abs(),
            OrderSide::Sell => total_volume / (worst_price - best_price).abs(),
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

        let depth_in_pips =
            if let (Some(best), Some(worst)) = (self.get_best_price(), self.get_worst_price()) {
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
}

/// Bid side of the order book (buy orders)
#[derive(Debug)]
pub struct BidSide(BookSide);

impl BidSide {
    /// Create a new bid side
    #[inline]
    pub fn new(buffer_size: usize) -> Self {
        Self(BookSide::new(OrderSide::Buy, buffer_size))
    }
}

impl Side for BidSide {
    #[inline]
    fn add_order(&mut self, order: Order) -> OrderId {
        assert_eq!(
            order.side,
            OrderSide::Buy,
            "Order side must be Buy for BidSide"
        );

        let id = order.id;
        let price = order.price;
        let quantity = order.remaining_quantity;

        // Get or create price level
        let price_level = self
            .0
            .price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel::new(price, self.0.buffer_size));

        // Add order to price level
        let order_id = price_level.add_order(order);

        // Update order map
        self.0.order_map.insert(order_id, price);

        // Update total volume
        self.0.total_volume += quantity;

        // Update best price cache
        if self.0.best_price_cache.is_none() || price >= self.0.best_price_cache.unwrap() {
            self.0.best_price_cache = Some(price);
        }

        id
    }

    #[inline]
    fn remove_order(&mut self, order_id: &OrderId) -> Option<Order> {
        // Get price from order map
        let price = *self.0.order_map.get(order_id)?;

        // Get price level
        let price_level = self.0.price_levels.get_mut(&price)?;

        // Remove order from price level
        let order = price_level.remove_order_by_id(order_id)?;

        // Update order map
        self.0.order_map.remove(order_id);

        // Update total volume
        self.0.total_volume -= order.remaining_quantity;

        // Remove price level if empty
        if price_level.is_empty() {
            self.0.price_levels.remove(&price);

            // Update best price cache if we removed the best price
            if Some(price) == self.0.best_price_cache {
                self.0.update_best_price_cache();
            }
        }

        Some(order)
    }

    #[inline]
    fn best_price(&self) -> Option<Price> {
        // For bid side, the best price is the highest (last key in descending order)
        self.0.get_best_price()
    }

    #[inline]
    fn total_volume(&self) -> Quantity {
        self.0.total_volume
    }

    #[inline]
    fn depth(&self) -> usize {
        self.0.price_levels.len()
    }

    #[inline]
    fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Get price from order map
        let price = *self.0.order_map.get(order_id)?;

        // Get price level
        let price_level = self.0.price_levels.get(&price)?;

        // Get order from price level
        price_level.get_order_by_id(order_id)
    }

    #[inline]
    fn update_order(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool {
        // Get price from order map
        if let Some(&price) = self.0.order_map.get(order_id) {
            // Get price level
            if let Some(price_level) = self.0.price_levels.get_mut(&price) {
                // Update order in price level
                let result = price_level.update_order_by_id(order_id, updater);

                // Recalculate total volume
                if result {
                    self.0.total_volume = self
                        .0
                        .price_levels
                        .values()
                        .map(|level| level.total_quantity())
                        .sum();
                }

                return result;
            }
        }

        false
    }

    #[inline]
    fn price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        // For bid side, price levels are sorted in descending order
        self.0.get_price_levels()
    }

    #[inline]
    fn get_price_level(&self, price: Price) -> Option<PriceLevelData> {
        self.0.price_levels.get(&price).map(|level| level.to_data())
    }

    #[inline]
    fn next_best_order(&mut self) -> Option<Order> {
        // Get best price
        let best_price = self.best_price()?;

        // Get best price level
        let price_level = self.0.price_levels.get_mut(&best_price)?;

        // Get next order at this price level
        if let Some((_, order)) = price_level.next_order() {
            // If this was the last order at this price level, update the best price cache
            if price_level.is_empty() {
                self.0.price_levels.remove(&best_price);
                self.0.update_best_price_cache();
            }

            // Update order map and total volume
            self.0.order_map.remove(&order.id);
            self.0.total_volume -= order.remaining_quantity;

            Some(order)
        } else {
            None
        }
    }
}

/// Ask side of the order book (sell orders)
#[derive(Debug)]
pub struct AskSide(BookSide);

impl AskSide {
    /// Create a new ask side
    #[inline]
    pub fn new(buffer_size: usize) -> Self {
        Self(BookSide::new(OrderSide::Sell, buffer_size))
    }
}

impl Side for AskSide {
    #[inline]
    fn add_order(&mut self, order: Order) -> OrderId {
        assert_eq!(
            order.side,
            OrderSide::Sell,
            "Order side must be Sell for AskSide"
        );

        let id = order.id;
        let price = order.price;
        let quantity = order.remaining_quantity;

        // Get or create price level
        let price_level = self
            .0
            .price_levels
            .entry(price)
            .or_insert_with(|| PriceLevel::new(price, self.0.buffer_size));

        // Add order to price level
        let order_id = price_level.add_order(order);

        // Update order map
        self.0.order_map.insert(order_id, price);

        // Update total volume
        self.0.total_volume += quantity;

        // Update best price cache
        if self.0.best_price_cache.is_none() || price <= self.0.best_price_cache.unwrap() {
            self.0.best_price_cache = Some(price);
        }

        id
    }

    #[inline]
    fn remove_order(&mut self, order_id: &OrderId) -> Option<Order> {
        // Get price from order map
        let price = *self.0.order_map.get(order_id)?;

        // Get price level
        let price_level = self.0.price_levels.get_mut(&price)?;

        // Remove order from price level
        let order = price_level.remove_order_by_id(order_id)?;

        // Update order map
        self.0.order_map.remove(order_id);

        // Update total volume
        self.0.total_volume -= order.remaining_quantity;

        // Remove price level if empty
        if price_level.is_empty() {
            self.0.price_levels.remove(&price);

            // Update best price cache if we removed the best price
            if Some(price) == self.0.best_price_cache {
                self.0.update_best_price_cache();
            }
        }

        Some(order)
    }

    #[inline]
    fn best_price(&self) -> Option<Price> {
        // For ask side, the best price is the lowest (first key in ascending order)
        self.0.get_best_price()
    }

    #[inline]
    fn total_volume(&self) -> Quantity {
        self.0.total_volume
    }

    #[inline]
    fn depth(&self) -> usize {
        self.0.price_levels.len()
    }

    #[inline]
    fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Get price from order map
        let price = *self.0.order_map.get(order_id)?;

        // Get price level
        let price_level = self.0.price_levels.get(&price)?;

        // Get order from price level
        price_level.get_order_by_id(order_id)
    }

    #[inline]
    fn update_order(&mut self, order_id: &OrderId, updater: impl FnOnce(&mut Order)) -> bool {
        // Get price from order map
        if let Some(&price) = self.0.order_map.get(order_id) {
            // Get price level
            if let Some(price_level) = self.0.price_levels.get_mut(&price) {
                // Update order in price level
                let result = price_level.update_order_by_id(order_id, updater);

                // Recalculate total volume
                if result {
                    self.0.total_volume = self
                        .0
                        .price_levels
                        .values()
                        .map(|level| level.total_quantity())
                        .sum();
                }

                return result;
            }
        }

        false
    }

    #[inline]
    fn price_levels(&self) -> Vec<(Price, PriceLevelData)> {
        // For ask side, price levels are sorted in ascending order
        self.0.get_price_levels()
    }

    #[inline]
    fn get_price_level(&self, price: Price) -> Option<PriceLevelData> {
        self.0.price_levels.get(&price).map(|level| level.to_data())
    }

    #[inline]
    fn next_best_order(&mut self) -> Option<Order> {
        // Get best price
        let best_price = self.best_price()?;

        // Get best price level
        let price_level = self.0.price_levels.get_mut(&best_price)?;

        // Get next order at this price level
        if let Some((_, order)) = price_level.next_order() {
            // If this was the last order at this price level, update the best price cache
            if price_level.is_empty() {
                self.0.price_levels.remove(&best_price);
                self.0.update_best_price_cache();
            }

            // Update order map and total volume
            self.0.order_map.remove(&order.id);
            self.0.total_volume -= order.remaining_quantity;

            Some(order)
        } else {
            None
        }
    }
}
