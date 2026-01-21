///
/// naml Runtime
///
/// This module provides the runtime support for naml programs compiled with
/// Cranelift JIT. It includes:
///
/// - Value representation (tagged union for dynamic typing at runtime boundaries)
/// - Reference-counted memory management
/// - Array operations
/// - String operations
/// - Struct field access
///
/// Design: All heap objects use atomic reference counting for thread safety.
/// Values are passed as 64-bit tagged pointers or inline primitives.
///

pub mod value;
pub mod array;
pub mod scheduler;
pub mod channel;

pub use value::*;
pub use array::*;
pub use scheduler::*;
pub use channel::*;

use std::io::Write;

/// Initialize the runtime (call once at program start)
pub fn init() {
    // Ensure stdout is line-buffered for print statements
    let _ = std::io::stdout().flush();
}
