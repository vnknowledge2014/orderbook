//! Utility functions and types.

use std::time::{Duration, Instant};

/// Measure the execution time of a function
pub fn measure_time<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    
    (result, elapsed)
}

/// Format a duration as microseconds
pub fn format_micros(duration: Duration) -> f64 {
    duration.as_secs() as f64 * 1_000_000.0 + duration.subsec_nanos() as f64 / 1_000.0
}

/// Format a duration as nanoseconds
pub fn format_nanos(duration: Duration) -> f64 {
    duration.as_secs() as f64 * 1_000_000_000.0 + duration.subsec_nanos() as f64
}
