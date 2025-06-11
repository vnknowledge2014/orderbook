//! Order generator for simulations.

use rand::Rng;
use rand::distributions::{Distribution, Uniform};
use crate::order_book::{
    Order, OrderSide, OrderType, TimeInForce,
    price_to_internal
};

/// Order generator configuration
#[derive(Debug, Clone)]  // Added Clone trait explicitly
pub struct OrderGeneratorConfig {
    /// Price range
    pub price_range: (f64, f64),
    /// Quantity range
    pub quantity_range: (f64, f64),
    /// Probability of buy orders
    pub buy_probability: f64,
    /// Probability of limit orders
    pub limit_probability: f64,
    /// Probability of GTC orders
    pub gtc_probability: f64,
    /// Number of clients
    pub num_clients: usize,
    /// Symbol
    pub symbol: String,
}

impl Default for OrderGeneratorConfig {
    fn default() -> Self {
        Self {
            price_range: (9900.0, 10100.0),
            quantity_range: (0.01, 10.0),
            buy_probability: 0.5,
            limit_probability: 0.8,
            gtc_probability: 0.7,
            num_clients: 100,
            symbol: "BTCUSD".to_string(),
        }
    }
}

/// Order generator for simulations
#[derive(Debug)]
pub struct OrderGenerator {
    /// Configuration
    config: OrderGeneratorConfig,
    /// Random number generator
    rng: rand::rngs::ThreadRng,
    /// Price distribution
    price_dist: Uniform<f64>,
    /// Quantity distribution
    quantity_dist: Uniform<f64>,
}

impl OrderGenerator {
    /// Create a new order generator
    pub fn new(config: OrderGeneratorConfig) -> Self {
        let rng = rand::thread_rng();
        let price_dist = Uniform::new(config.price_range.0, config.price_range.1);
        let quantity_dist = Uniform::new(config.quantity_range.0, config.quantity_range.1);
        
        Self {
            config,
            rng,
            price_dist,
            quantity_dist,
        }
    }
    
    /// Generate a random order
    pub fn generate_order(&mut self) -> Order {
        // Generate random parameters
        let side = if self.rng.gen::<f64>() < self.config.buy_probability {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        
        let order_type = if self.rng.gen::<f64>() < self.config.limit_probability {
            OrderType::Limit
        } else {
            OrderType::Market
        };
        
        let time_in_force = if self.rng.gen::<f64>() < self.config.gtc_probability {
            TimeInForce::GTC
        } else {
            TimeInForce::IOC
        };
        
        let price = self.price_dist.sample(&mut self.rng);
        let quantity = self.quantity_dist.sample(&mut self.rng);
        
        let client_id = format!("client{}", self.rng.gen_range(0..self.config.num_clients));
        
        // Create the order
        match order_type {
            OrderType::Limit => {
                Order::new_limit(
                    side,
                    price_to_internal(price),
                    quantity,
                    client_id,
                    time_in_force,
                )
            }
            OrderType::Market => {
                Order::new_market(
                    side,
                    quantity,
                    client_id,
                )
            }
            _ => {
                // Default to limit order
                Order::new_limit(
                    side,
                    price_to_internal(price),
                    quantity,
                    client_id,
                    time_in_force,
                )
            }
        }
    }
    
    /// Generate multiple orders
    pub fn generate_orders(&mut self, count: usize) -> Vec<Order> {
        (0..count).map(|_| self.generate_order()).collect()
    }
}
