//!
//! naml Runtime
//!
//! This module provides the runtime support for naml programs compiled with
//! Cranelift JIT. It includes:
//!
//! - Value representation (tagged union for dynamic typing at runtime boundaries)
//! - Reference-counted memory management
//! - Array operations
//! - String operations
//! - Struct field access
//! - Exception handling support
//!
//! Design: All heap objects use atomic reference counting for thread safety.
//! Values are passed as 64-bit tagged pointers or inline primitives.
//!

pub mod value;
pub mod array;
pub mod scheduler;
pub mod channel;
pub mod map;

pub use value::*;
pub use array::*;
pub use scheduler::*;
pub use channel::*;
pub use map::*;

use std::cell::Cell;
use std::io::Write;

thread_local! {
    static CURRENT_EXCEPTION: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) };
}

/// Initialize the runtime (call once at program start)
pub fn init() {
    // Ensure stdout is line-buffered for print statements
    let _ = std::io::stdout().flush();
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
