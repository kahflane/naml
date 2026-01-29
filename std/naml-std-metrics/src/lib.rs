//!
//! naml-std-metrics - Performance Measurement Utilities
//!
//! Provides high-resolution timing for benchmarking naml programs.
//!
//! ## Functions
//!
//! - `perf_now() -> int` - High-resolution monotonic time in nanoseconds
//! - `elapsed_ms(start_ns: int) -> int` - Milliseconds elapsed since start
//! - `elapsed_us(start_ns: int) -> int` - Microseconds elapsed since start
//! - `elapsed_ns(start_ns: int) -> int` - Nanoseconds elapsed since start
//!
//! ## Example
//!
//! ```naml
//! use std::metrics::*;
//!
//! fn main() {
//!     var start: int = perf_now();
//!     // ... work ...
//!     println("Took {} ms", elapsed_ms(start));
//! }
//! ```
//!

use std::time::Instant;
use std::sync::OnceLock;

static START_INSTANT: OnceLock<Instant> = OnceLock::new();

fn get_start() -> &'static Instant {
    START_INSTANT.get_or_init(Instant::now)
}

/// Get high-resolution monotonic time in nanoseconds
/// Returns nanoseconds since an arbitrary but consistent starting point
#[unsafe(no_mangle)]
pub extern "C" fn naml_metrics_perf_now() -> i64 {
    let start = get_start();
    start.elapsed().as_nanos() as i64
}

/// Calculate milliseconds elapsed since start_ns
#[unsafe(no_mangle)]
pub extern "C" fn naml_metrics_elapsed_ms(start_ns: i64) -> i64 {
    let now = naml_metrics_perf_now();
    (now - start_ns) / 1_000_000
}

/// Calculate microseconds elapsed since start_ns
#[unsafe(no_mangle)]
pub extern "C" fn naml_metrics_elapsed_us(start_ns: i64) -> i64 {
    let now = naml_metrics_perf_now();
    (now - start_ns) / 1_000
}

/// Calculate nanoseconds elapsed since start_ns
#[unsafe(no_mangle)]
pub extern "C" fn naml_metrics_elapsed_ns(start_ns: i64) -> i64 {
    let now = naml_metrics_perf_now();
    now - start_ns
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_perf_now_monotonic() {
        let t1 = naml_metrics_perf_now();
        thread::sleep(Duration::from_millis(1));
        let t2 = naml_metrics_perf_now();
        assert!(t2 > t1);
    }

    #[test]
    fn test_elapsed_ms() {
        let start = naml_metrics_perf_now();
        thread::sleep(Duration::from_millis(10));
        let elapsed = naml_metrics_elapsed_ms(start);
        assert!(elapsed >= 9 && elapsed < 50);
    }

    #[test]
    fn test_elapsed_us() {
        let start = naml_metrics_perf_now();
        thread::sleep(Duration::from_millis(1));
        let elapsed = naml_metrics_elapsed_us(start);
        assert!(elapsed >= 900 && elapsed < 50000);
    }

    #[test]
    fn test_elapsed_ns() {
        let start = naml_metrics_perf_now();
        thread::sleep(Duration::from_millis(1));
        let elapsed = naml_metrics_elapsed_ns(start);
        assert!(elapsed >= 900_000 && elapsed < 50_000_000);
    }
}
