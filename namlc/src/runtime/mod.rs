//!
//! naml Runtime
//!
//! This module provides the runtime support for naml programs compiled with
//! Cranelift JIT. It re-exports functions from the separate standard library
//! crates and includes additional runtime components:
//!
//! - Value representation (from naml-std-core)
//! - Random number generation (from naml-std-random)
//! - I/O operations (from naml-std-io)
//! - Threading and channels (from naml-std-threads)
//! - Array operations (local)
//! - Map operations (local)
//! - Bytes operations (local)
//!

pub mod array;
pub mod map;
pub mod bytes;

pub use naml_std_core::*;
pub use naml_std_random::*;
pub use naml_std_io::*;
pub use naml_std_threads::*;

pub use array::*;
pub use map::*;
pub use bytes::*;

/// Initialize the runtime (call once at program start)
pub fn init() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
