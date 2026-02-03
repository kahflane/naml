//!
//! Exception Handling Primitives
//!
//! Provides thread-local exception storage for try/catch support in naml.
//! Exceptions are stored as raw pointers and managed by the generated code.
//!
//! Exception Type IDs:
//! - 0: Unknown/User-defined exception
//! - 1: IOError
//! - 2: PermissionError
//! - 3: DecodeError
//! - 4: PathError
//! - 5: NetworkError
//! - 6: TimeoutError
//!

use std::cell::Cell;

thread_local! {
    static CURRENT_EXCEPTION: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) };
    static CURRENT_EXCEPTION_TYPE_ID: Cell<i64> = const { Cell::new(0) };
}

/// Exception type IDs for built-in exceptions
pub const EXCEPTION_TYPE_UNKNOWN: i64 = 0;
pub const EXCEPTION_TYPE_IO_ERROR: i64 = 1;
pub const EXCEPTION_TYPE_PERMISSION_ERROR: i64 = 2;
pub const EXCEPTION_TYPE_DECODE_ERROR: i64 = 3;
pub const EXCEPTION_TYPE_PATH_ERROR: i64 = 4;
pub const EXCEPTION_TYPE_NETWORK_ERROR: i64 = 5;
pub const EXCEPTION_TYPE_TIMEOUT_ERROR: i64 = 6;

/// Set the current exception (called by throw)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_set(exception_ptr: *mut u8) {
    CURRENT_EXCEPTION.with(|ex| ex.set(exception_ptr));
}

/// Set the current exception with type ID
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_set_typed(exception_ptr: *mut u8, type_id: i64) {
    CURRENT_EXCEPTION.with(|ex| ex.set(exception_ptr));
    CURRENT_EXCEPTION_TYPE_ID.with(|id| id.set(type_id));
}

/// Get the current exception type ID
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_get_type_id() -> i64 {
    CURRENT_EXCEPTION_TYPE_ID.with(|id| id.get())
}

/// Check if current exception matches the given type ID
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_is_type(type_id: i64) -> i64 {
    let current = CURRENT_EXCEPTION_TYPE_ID.with(|id| id.get());
    if current == type_id { 1 } else { 0 }
}

/// Get the current exception pointer (0 if none)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_get() -> *mut u8 {
    CURRENT_EXCEPTION.with(|ex| ex.get())
}

/// Clear the current exception (called after catch handles it)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_clear() {
    CURRENT_EXCEPTION.with(|ex| ex.set(std::ptr::null_mut()));
    CURRENT_EXCEPTION_TYPE_ID.with(|id| id.set(0));
}

/// Clear only the exception pointer, preserving type ID for 'is' checks in catch blocks
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_clear_ptr() {
    CURRENT_EXCEPTION.with(|ex| ex.set(std::ptr::null_mut()));
}

/// Check if there's a pending exception
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_check() -> i64 {
    CURRENT_EXCEPTION.with(|ex| if ex.get().is_null() { 0 } else { 1 })
}
