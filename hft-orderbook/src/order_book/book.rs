//! Main order book implementation.

use crate::order_book::market_depth::{
    ExecutionLevel, MarketDepthData, MarketDepthOptions, MarketDepthStatistics, MarketImpactData,
    PriceVolumeLevel,
};
use crate::order_book::price_level::PriceLevelData;
use crate::order_book::{
    AskSide, BidSide, ClientId, Order, OrderId, OrderSide, Price, Quantity, Side, TimeInForce,
    Timestamp,
};
use serde::{Deserialize, Serialize};

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
    #[inline]
    pub fn new(symbol: String, exchange_id: String, config: OrderBookConfig) -> Self {
        Self {
            symbol,
            exchange_id,
            bid_side: BidSide::new(config.buffer_size),
            ask_side: AskSide::new(config.buffer_size),
            last_update_id: 0,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64,
            config,
        }
    }

    /// Add a limit order to the book
    #[inline]
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
                self.bid_side.add_order(order);
            }
            OrderSide::Sell => {
                self.ask_side.add_order(order);
            }
        }

        // Update timestamp and ID
        self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        self.last_update_id += 1;

        Ok(order_id)
    }

    /// Cancel an order
    #[inline]
    pub fn cancel_order(&mut self, order_id: &OrderId) -> Result<Order, OrderBookError> {
        // Try to find and remove the order from either side
        if let Some(order) = self.bid_side.remove_order(order_id) {
            // Update timestamp and ID
            self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            self.last_update_id += 1;

            Ok(order)
        } else if let Some(order) = self.ask_side.remove_order(order_id) {
            // Update timestamp and ID
            self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            self.last_update_id += 1;

            Ok(order)
        } else {
            Err(OrderBookError::OrderNotFound)
        }
    }

    /// Get the best bid price
    #[inline]
    pub fn best_bid(&self) -> Option<Price> {
        self.bid_side.best_price()
    }

    /// Get the best ask price
    #[inline]
    pub fn best_ask(&self) -> Option<Price> {
        self.ask_side.best_price()
    }

    /// Get the next best bid order (for matching)
    #[inline]
    pub fn next_best_bid(&mut self) -> Option<Order> {
        let order = self.bid_side.next_best_order();

        if order.is_some() {
            // Update timestamp and ID
            self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            self.last_update_id += 1;
        }

        order
    }

    /// Get the next best ask order (for matching)
    #[inline]
    pub fn next_best_ask(&mut self) -> Option<Order> {
        let order = self.ask_side.next_best_order();

        if order.is_some() {
            // Update timestamp and ID
            self.timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
            self.last_update_id += 1;
        }

        order
    }

    /// Get the spread
    #[inline]
    pub fn spread(&self) -> Option<Price> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price
    #[inline]
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
    #[inline]
    pub fn get_order(&self, order_id: &OrderId) -> Option<Order> {
        // Try to find the order on either side
        self.bid_side
            .get_order(order_id)
            .or_else(|| self.ask_side.get_order(order_id))
    }

    /// Get the top N bid levels
    #[inline]
    pub fn top_bids(&self, n: usize) -> Vec<(Price, Quantity)> {
        self.bid_side
            .price_levels()
            .into_iter()
            .take(n)
            .map(|(price, level)| (price, level.total_quantity))
            .collect()
    }

    /// Get the top N ask levels
    #[inline]
    pub fn top_asks(&self, n: usize) -> Vec<(Price, Quantity)> {
        self.ask_side
            .price_levels()
            .into_iter()
            .take(n)
            .map(|(price, level)| (price, level.total_quantity))
            .collect()
    }

    /// Lấy dữ liệu market depth
    pub fn get_market_depth(&self, options: Option<MarketDepthOptions>) -> MarketDepthData {
        let options = options.unwrap_or_default();

        // Lấy tất cả các mức giá từ cả hai phía
        let bid_levels = self.bid_side.price_levels();
        let ask_levels = self.ask_side.price_levels();

        // Tính toán tổng khối lượng
        let total_bid_volume: Quantity = bid_levels
            .iter()
            .map(|(_, level)| level.total_quantity)
            .sum();

        let total_ask_volume: Quantity = ask_levels
            .iter()
            .map(|(_, level)| level.total_quantity)
            .sum();

        // Xử lý bid levels
        let mut processed_bids = Vec::new();
        let mut cumulative_bid_volume = 0.0;

        for (i, (price, level_data)) in bid_levels.into_iter().enumerate() {
            if i >= options.max_levels {
                break;
            }

            cumulative_bid_volume += level_data.total_quantity;

            processed_bids.push(PriceVolumeLevel {
                price,
                volume: level_data.total_quantity,
                order_count: level_data.order_count,
                cumulative_volume: if options.calculate_cumulative {
                    cumulative_bid_volume
                } else {
                    0.0
                },
                percentage_of_total: level_data.total_quantity / total_bid_volume * 100.0,
            });
        }

        // Xử lý ask levels
        let mut processed_asks = Vec::new();
        let mut cumulative_ask_volume = 0.0;

        for (i, (price, level_data)) in ask_levels.into_iter().enumerate() {
            if i >= options.max_levels {
                break;
            }

            cumulative_ask_volume += level_data.total_quantity;

            processed_asks.push(PriceVolumeLevel {
                price,
                volume: level_data.total_quantity,
                order_count: level_data.order_count,
                cumulative_volume: if options.calculate_cumulative {
                    cumulative_ask_volume
                } else {
                    0.0
                },
                percentage_of_total: level_data.total_quantity / total_ask_volume * 100.0,
            });
        }

        // Tính toán các thống kê
        let statistics = if options.calculate_statistics {
            self.calculate_market_depth_statistics(
                &processed_bids,
                &processed_asks,
                total_bid_volume,
                total_ask_volume,
            )
        } else {
            MarketDepthStatistics {
                vwap: 0.0,
                spread: 0.0,
                spread_bps: 0.0,
                bid_ask_ratio: 0.0,
                imbalance: 0.0,
                liquidity_index: 0.0,
            }
        };

        MarketDepthData {
            timestamp: self.timestamp,
            bids: processed_bids,
            asks: processed_asks,
            statistics,
        }
    }

    /// Tính toán các thống kê market depth
    fn calculate_market_depth_statistics(
        &self,
        bids: &[PriceVolumeLevel],
        asks: &[PriceVolumeLevel],
        total_bid_volume: Quantity,
        total_ask_volume: Quantity,
    ) -> MarketDepthStatistics {
        // Tính VWAP (Volume Weighted Average Price)
        let bid_vwap = if total_bid_volume > 0.0 {
            bids.iter()
                .map(|level| {
                    (level.price as f64 / crate::order_book::PRICE_MULTIPLIER as f64) * level.volume
                })
                .sum::<f64>()
                / total_bid_volume
        } else {
            0.0
        };

        let ask_vwap = if total_ask_volume > 0.0 {
            asks.iter()
                .map(|level| {
                    (level.price as f64 / crate::order_book::PRICE_MULTIPLIER as f64) * level.volume
                })
                .sum::<f64>()
                / total_ask_volume
        } else {
            0.0
        };

        let vwap = (bid_vwap + ask_vwap) / 2.0;

        // Tính spread
        let spread = match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => {
                let ask_f = ask as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                let bid_f = bid as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
                ask_f - bid_f
            }
            _ => 0.0,
        };

        // Tính spread bps
        let mid_price = self.mid_price().unwrap_or(0.0);
        let spread_bps = if mid_price > 0.0 {
            (spread / mid_price) * 10000.0
        } else {
            0.0
        };

        // Tính bid/ask ratio
        let bid_ask_ratio = if total_ask_volume > 0.0 {
            total_bid_volume / total_ask_volume
        } else {
            0.0
        };

        // Tính imbalance
        let imbalance = if total_bid_volume + total_ask_volume > 0.0 {
            (total_bid_volume - total_ask_volume) / (total_bid_volume + total_ask_volume)
        } else {
            0.0
        };

        // Tính liquidity index (đơn giản hóa)
        let top_5_bid_volume: Quantity = bids.iter().take(5).map(|level| level.volume).sum();
        let top_5_ask_volume: Quantity = asks.iter().take(5).map(|level| level.volume).sum();
        let liquidity_index = (top_5_bid_volume + top_5_ask_volume) / 2.0;

        MarketDepthStatistics {
            vwap,
            spread,
            spread_bps,
            bid_ask_ratio,
            imbalance,
            liquidity_index,
        }
    }

    /// Tính toán tác động thị trường cho một lệnh với kích thước cho trước
    pub fn calculate_market_impact(
        &self,
        side: OrderSide,
        order_size: Quantity,
    ) -> MarketImpactData {
        let mut remaining_size = order_size;
        let mut execution_levels = Vec::new();
        let mut total_value = 0.0;

        // Xác định phía cần lấy mức giá
        let price_levels = match side {
            OrderSide::Buy => self.ask_side.price_levels(),
            OrderSide::Sell => self.bid_side.price_levels(),
        };

        // Duyệt qua các mức giá cho đến khi lệnh được thực hiện hoàn toàn
        for (price, level_data) in price_levels {
            let price_f = price as f64 / crate::order_book::PRICE_MULTIPLIER as f64;
            let available_volume = level_data.total_quantity;

            // Tính khối lượng thực hiện ở mức giá này
            let executed_at_this_level = remaining_size.min(available_volume);

            if executed_at_this_level > 0.0 {
                // Cập nhật tổng giá trị
                total_value += price_f * executed_at_this_level;

                // Thêm mức thực hiện vào danh sách
                execution_levels.push(ExecutionLevel {
                    price,
                    executed_volume: executed_at_this_level,
                    percentage_of_order: executed_at_this_level / order_size * 100.0,
                });

                // Cập nhật khối lượng còn lại
                remaining_size -= executed_at_this_level;

                // Nếu đã thực hiện hết lệnh, thoát khỏi vòng lặp
                if remaining_size <= 0.0 {
                    break;
                }
            }
        }

        // Tính giá thực hiện trung bình
        let expected_execution_price = if order_size - remaining_size > 0.0 {
            total_value / (order_size - remaining_size)
        } else {
            0.0
        };

        // Tính tác động giá
        let base_price = match side {
            OrderSide::Buy => self.best_ask(),
            OrderSide::Sell => self.best_bid(),
        };

        let expected_price_impact_percent = if let Some(base) = base_price {
            let base_f = base as f64 / crate::order_book::PRICE_MULTIPLIER as f64;

            match side {
                OrderSide::Buy => {
                    if base_f > 0.0 {
                        (expected_execution_price - base_f) / base_f * 100.0
                    } else {
                        0.0
                    }
                }
                OrderSide::Sell => {
                    if base_f > 0.0 {
                        (base_f - expected_execution_price) / base_f * 100.0
                    } else {
                        0.0
                    }
                }
            }
        } else {
            0.0
        };

        // Tính slippage
        let expected_slippage = match side {
            OrderSide::Buy => expected_price_impact_percent,
            OrderSide::Sell => expected_price_impact_percent,
        };

        // Tính chi phí thực hiện
        let estimated_execution_cost =
            expected_slippage / 100.0 * order_size * expected_execution_price;

        MarketImpactData {
            order_size,
            side,
            expected_execution_price,
            expected_price_impact_percent,
            expected_slippage,
            estimated_execution_cost,
            execution_levels,
        }
    }

    /// Lấy thông tin về liquidity profile của order book
    pub fn get_liquidity_profile(&self, price_range_percent: f64) -> LiquidityProfile {
        let mid_price = match self.mid_price() {
            Some(price) => price,
            None => return LiquidityProfile::default(),
        };

        // Tính range dựa trên mid price
        let price_range = mid_price * price_range_percent / 100.0;
        let min_price = mid_price - price_range;
        let max_price = mid_price + price_range;

        // Chuyển đổi về định dạng giá nội bộ
        let min_price_internal = crate::order_book::price_to_internal(min_price);
        let max_price_internal = crate::order_book::price_to_internal(max_price);

        // Tính tổng khối lượng và số lệnh trong range
        let mut bid_volume_in_range = 0.0;
        let mut ask_volume_in_range = 0.0;
        let mut bid_orders_in_range = 0;
        let mut ask_orders_in_range = 0;

        // Kiểm tra phía bid
        for (price, level_data) in self.bid_side.price_levels() {
            if price >= min_price_internal {
                bid_volume_in_range += level_data.total_quantity;
                bid_orders_in_range += level_data.order_count;
            }
        }

        // Kiểm tra phía ask
        for (price, level_data) in self.ask_side.price_levels() {
            if price <= max_price_internal {
                ask_volume_in_range += level_data.total_quantity;
                ask_orders_in_range += level_data.order_count;
            }
        }

        // Tính các chỉ số khác
        let total_volume_in_range = bid_volume_in_range + ask_volume_in_range;
        let total_orders_in_range = bid_orders_in_range + ask_orders_in_range;

        let bid_concentration = if bid_volume_in_range > 0.0 {
            let total_bid_volume: Quantity = self
                .bid_side
                .price_levels()
                .iter()
                .map(|(_, level)| level.total_quantity)
                .sum();

            bid_volume_in_range / total_bid_volume
        } else {
            0.0
        };

        let ask_concentration = if ask_volume_in_range > 0.0 {
            let total_ask_volume: Quantity = self
                .ask_side
                .price_levels()
                .iter()
                .map(|(_, level)| level.total_quantity)
                .sum();

            ask_volume_in_range / total_ask_volume
        } else {
            0.0
        };

        LiquidityProfile {
            mid_price,
            price_range_percent,
            min_price,
            max_price,
            bid_volume_in_range,
            ask_volume_in_range,
            total_volume_in_range,
            bid_orders_in_range,
            ask_orders_in_range,
            total_orders_in_range,
            bid_concentration,
            ask_concentration,
        }
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

        // Create bid side data
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
                order_count: self
                    .bid_side
                    .price_levels()
                    .iter()
                    .map(|(_, level)| level.order_count)
                    .sum(),
                depth_in_pips: 0, // TODO: Implement this properly
            },
        };

        // Create ask side data
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
                order_count: self
                    .ask_side
                    .price_levels()
                    .iter()
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
    #[inline]
    fn validate_order(&self, _price: Price, quantity: Quantity) -> Result<(), OrderBookError> {
        // Check if the order size is valid
        if quantity < self.config.min_order_size {
            return Err(OrderBookError::InvalidOrderSize(
                "Order size too small".to_string(),
            ));
        }

        if quantity > self.config.max_order_size {
            return Err(OrderBookError::InvalidOrderSize(
                "Order size too large".to_string(),
            ));
        }

        // Check if the order book is full
        if self.bid_side.depth() + self.ask_side.depth() >= self.config.max_levels {
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

/// Thông tin về liquidity profile của order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityProfile {
    /// Giá trung bình
    pub mid_price: f64,
    /// Phần trăm khoảng giá được xem xét
    pub price_range_percent: f64,
    /// Giá thấp nhất trong khoảng
    pub min_price: f64,
    /// Giá cao nhất trong khoảng
    pub max_price: f64,
    /// Tổng khối lượng bid trong khoảng
    pub bid_volume_in_range: Quantity,
    /// Tổng khối lượng ask trong khoảng
    pub ask_volume_in_range: Quantity,
    /// Tổng khối lượng trong khoảng
    pub total_volume_in_range: Quantity,
    /// Số lệnh bid trong khoảng
    pub bid_orders_in_range: usize,
    /// Số lệnh ask trong khoảng
    pub ask_orders_in_range: usize,
    /// Tổng số lệnh trong khoảng
    pub total_orders_in_range: usize,
    /// Tỷ lệ khối lượng bid trong khoảng so với tổng khối lượng bid
    pub bid_concentration: f64,
    /// Tỷ lệ khối lượng ask trong khoảng so với tổng khối lượng ask
    pub ask_concentration: f64,
}

impl Default for LiquidityProfile {
    fn default() -> Self {
        Self {
            mid_price: 0.0,
            price_range_percent: 0.0,
            min_price: 0.0,
            max_price: 0.0,
            bid_volume_in_range: 0.0,
            ask_volume_in_range: 0.0,
            total_volume_in_range: 0.0,
            bid_orders_in_range: 0,
            ask_orders_in_range: 0,
            total_orders_in_range: 0,
            bid_concentration: 0.0,
            ask_concentration: 0.0,
        }
    }
}
