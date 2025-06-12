use serde::{Serialize, Deserialize};
use crate::order_book::{Price, Quantity, OrderSide, OrderBook};

/// Dữ liệu market depth (độ sâu thị trường)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepthData {
    /// Thời điểm dữ liệu được thu thập
    pub timestamp: u64,
    /// Bid levels theo mức giá giảm dần
    pub bids: Vec<PriceVolumeLevel>,
    /// Ask levels theo mức giá tăng dần
    pub asks: Vec<PriceVolumeLevel>,
    /// Thống kê liên quan đến market depth
    pub statistics: MarketDepthStatistics,
}

/// Một mức giá trong market depth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceVolumeLevel {
    /// Mức giá
    pub price: Price,
    /// Khối lượng tại mức giá
    pub volume: Quantity,
    /// Số lượng lệnh tại mức giá
    pub order_count: usize,
    /// Khối lượng tích lũy đến mức giá này
    pub cumulative_volume: Quantity,
    /// Phần trăm của tổng khối lượng
    pub percentage_of_total: f64,
}

/// Thống kê market depth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepthStatistics {
    /// Giá trung bình có trọng số khối lượng (VWAP)
    pub vwap: f64,
    /// Spread (chênh lệch giữa giá mua tốt nhất và giá bán tốt nhất)
    pub spread: f64,
    /// Spread tính theo basis points
    pub spread_bps: f64,
    /// Tỷ lệ khối lượng bid/ask
    pub bid_ask_ratio: f64,
    /// Mức độ chênh lệch (imbalance) giữa bid và ask
    pub imbalance: f64,
    /// Chỉ số thanh khoản (liquidity index)
    pub liquidity_index: f64,
}

/// Dữ liệu tác động thị trường (market impact)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketImpactData {
    /// Kích thước lệnh
    pub order_size: Quantity,
    /// Phía của lệnh (mua/bán)
    pub side: OrderSide,
    /// Giá trung bình thực hiện dự kiến
    pub expected_execution_price: f64,
    /// Tác động giá dự kiến (theo %)
    pub expected_price_impact_percent: f64,
    /// Trượt giá (slippage) dự kiến
    pub expected_slippage: f64,
    /// Chi phí thực hiện ước tính
    pub estimated_execution_cost: f64,
    /// Các mức giá và khối lượng dự kiến sẽ được ghép
    pub execution_levels: Vec<ExecutionLevel>,
}

/// Mức giá thực hiện trong tính toán tác động thị trường
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLevel {
    /// Mức giá
    pub price: Price,
    /// Khối lượng dự kiến sẽ được thực hiện ở mức giá này
    pub executed_volume: Quantity,
    /// Phần trăm của tổng lệnh
    pub percentage_of_order: f64,
}

/// Tùy chọn cho truy vấn market depth
#[derive(Debug, Clone)]
pub struct MarketDepthOptions {
    /// Số lượng mức giá tối đa cần lấy
    pub max_levels: usize,
    /// Có tính các thống kê không
    pub calculate_statistics: bool,
    /// Có tính khối lượng tích lũy không
    pub calculate_cumulative: bool,
    /// Có phân nhóm các mức giá gần nhau không
    pub group_levels: bool,
    /// Khoảng cách tối thiểu giữa các mức giá khi phân nhóm
    pub group_threshold: Option<Price>,
}

impl Default for MarketDepthOptions {
    fn default() -> Self {
        Self {
            max_levels: 10,
            calculate_statistics: true,
            calculate_cumulative: true,
            group_levels: false,
            group_threshold: None,
        }
    }
}