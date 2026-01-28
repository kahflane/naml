//!
//! Exception Handling Primitives
//!
//! Provides thread-local exception storage for try/catch support in naml.
//! Exceptions are stored as raw pointers and managed by the generated code.
//!

use std::cell::Cell;

thread_local! {
    static CURRENT_EXCEPTION: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) };
}

/// Set the current exception (called by throw)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_set(exception_ptr: *mut u8) {
    CURRENT_EXCEPTION.with(|ex| ex.set(exception_ptr));
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
}

/// Check if there's a pending exception
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_check() -> i64 {
    CURRENT_EXCEPTION.with(|ex| if ex.get().is_null() { 0 } else { 1 })
}
