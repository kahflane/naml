//!
//! naml-std-random - Random Number Generation
//!
//! Provides random number generation for naml programs using a fast XORshift
//! algorithm. The generator is automatically seeded from system time on first use.
//!
//! ## Functions
//!
//! - `random(min: int, max: int) -> int` - Random integer in range [min, max]
//! - `random_float() -> float` - Random float in range [0.0, 1.0)
//!
//! ## Thread Safety
//!
//! The RNG state is stored in an atomic variable, making it safe to use from
//! multiple threads. However, concurrent access may reduce randomness quality
//! slightly due to potential race conditions in the state update.
//!

use std::sync::atomic::{AtomicU64, Ordering};

static RNG_STATE: AtomicU64 = AtomicU64::new(0);

fn rng_next() -> u64 {
    let mut s = RNG_STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        if s == 0 { s = 1; }
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    RNG_STATE.store(s, Ordering::Relaxed);
    s
}

/// Generate a random integer in the range [min, max] (inclusive)
#[unsafe(no_mangle)]
pub extern "C" fn naml_random(min: i64, max: i64) -> i64 {
    if min >= max {
        return min;
    }
    let range = (max - min + 1) as u64;
    let r = rng_next() % range;
    min + r as i64
}

/// Generate a random float in the range [0.0, 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn naml_random_float() -> f64 {
    let r = rng_next();
    (r >> 11) as f64 / (1u64 << 53) as f64
}

/// Seed the random number generator with a specific value
#[unsafe(no_mangle)]
pub extern "C" fn naml_random_seed(seed: u64) {
    let s = if seed == 0 { 1 } else { seed };
    RNG_STATE.store(s, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_range() {
        for _ in 0..100 {
            let r = naml_random(10, 20);
            assert!(r >= 10 && r <= 20);
        }
    }

    #[test]
    fn test_random_same_min_max() {
        assert_eq!(naml_random(5, 5), 5);
        assert_eq!(naml_random(5, 4), 5);
    }

    #[test]
    fn test_random_float_range() {
        for _ in 0..100 {
            let r = naml_random_float();
            assert!(r >= 0.0 && r < 1.0);
        }
    }

    #[test]
    fn test_seeding() {
        naml_random_seed(12345);
        let a = naml_random(0, 1000);
        naml_random_seed(12345);
        let b = naml_random(0, 1000);
        assert_eq!(a, b);
    }
}
