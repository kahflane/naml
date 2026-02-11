///
/// naml Runtime Static Library
///
/// Produces a static library (libnaml_runtime.a) that gets linked with
/// AOT-compiled naml object files to produce standalone binaries.
///
/// All runtime functions are provided by naml-std-* crates; this crate
/// re-exports them so they appear as link-time symbols in the static lib.
///

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
pub use naml_std_net::*;
pub use naml_std_env::*;
pub use naml_std_os::*;
pub use naml_std_process::*;
pub use naml_std_testing::*;
pub use naml_std_sqlite3::*;
pub use naml_std_timers::*;
pub use naml_std_crypto::*;

pub use naml_std_collections::arrays::*;
pub use naml_std_collections::maps::{
    naml_map_count, naml_map_contains_key, naml_map_remove, naml_map_clear,
    naml_map_keys, naml_map_values, naml_map_entries, naml_map_first_key, naml_map_first_value,
    naml_map_any, naml_map_all, naml_map_count_if, naml_map_fold,
    naml_map_transform, naml_map_where, naml_map_reject,
    naml_map_merge, naml_map_defaults, naml_map_intersect, naml_map_diff,
    naml_map_invert, naml_map_from_arrays, naml_map_from_entries,
};

pub fn init() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}
