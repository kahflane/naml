///
/// naml Runtime Static Library
///
/// Provides all runtime functions needed by AOT-compiled naml programs.
/// This crate produces a static library (libnaml_runtime.a) that gets
/// linked with the compiled naml object file to produce a standalone binary.
///
/// Contains:
/// - Print builtins (naml_print_int, naml_print_float, etc.)
/// - Map operations (naml_map_new, naml_map_set, etc.)
/// - Bytes operations (naml_bytes_new, naml_bytes_from, etc.)
/// - All standard library functions via naml-std-* crate dependencies
///

mod map;
mod bytes;

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

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_int(val: i64) {
    print!("{}", val);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_float(val: f64) {
    print!("{}", val);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_bool(val: i64) {
    if val != 0 {
        print!("true");
    } else {
        print!("false");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_str(ptr: *const std::ffi::c_char) {
    if !ptr.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        if let Ok(s) = c_str.to_str() {
            print!("{}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_newline() {
    println!();
}
