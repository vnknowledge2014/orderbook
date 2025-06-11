//! Simulation utilities for the order book.

pub mod generator;  // Made public
mod benchmark;

pub use generator::OrderGenerator;
pub use generator::OrderGeneratorConfig;  // Export OrderGeneratorConfig
pub use benchmark::{run_simulation, SimulationConfig, SimulationMode};
