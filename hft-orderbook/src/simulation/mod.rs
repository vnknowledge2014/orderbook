//! Simulation utilities for the order book.

mod generator;
mod benchmark;

pub use generator::OrderGenerator;
pub use benchmark::{run_simulation, SimulationConfig, SimulationMode};
