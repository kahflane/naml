//!
//! naml Runtime
//!
//! This module provides the runtime support for naml programs compiled with
//! Cranelift JIT. It re-exports functions from the separate standard library
//! crates and includes additional runtime components:
//!
//! - Core types and arrays (from naml-std-core)
//! - Random number generation (from naml-std-random)
//! - I/O operations (from naml-std-io)
//! - Threading and channels (from naml-std-threads)
//! - String operations (from naml-std-strings)
//! - Collection operations (from naml-std-collections)
//! - File system operations (from naml-std-fs)
//! - Path operations (from naml-std-path)
//! - Encoding operations (from naml-std-encoding)
//! - Map operations (local)
//! - Bytes operations (local)
//!

pub mod map;
pub mod bytes;

pub use naml_std_core::*;
pub use naml_std_random::*;
pub use naml_std_io::*;
pub use naml_std_threads::*;
pub use naml_std_datetime::*;
pub use naml_std_metrics::*;
pub use naml_std_strings::*;
pub use naml_std_fs::*;
pub use naml_std_path::*;
pub use naml_std_encoding::*;

// Import collection arrays functions (not the arrays module to avoid conflict with naml_std_core::array)
pub use naml_std_collections::arrays::*;
// Import collection maps functions (not the maps module to avoid conflict with local map module)
pub use naml_std_collections::maps::{
    naml_map_count, naml_map_contains_key, naml_map_remove, naml_map_clear,
    naml_map_keys, naml_map_values, naml_map_entries, naml_map_first_key, naml_map_first_value,
    naml_map_any, naml_map_all, naml_map_count_if, naml_map_fold,
    naml_map_transform, naml_map_where, naml_map_reject,
    naml_map_merge, naml_map_defaults, naml_map_intersect, naml_map_diff,
    naml_map_invert, naml_map_from_arrays, naml_map_from_entries,
};

pub use map::*;
pub use bytes::*;

/// Initialize the runtime (call once at program start)
pub fn init() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
