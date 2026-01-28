//!
//! naml-std-core - Core Runtime Types
//!
//! This crate provides the fundamental types shared across all naml standard library crates:
//!
//! - `HeapHeader` and `HeapTag` for reference-counted heap objects
//! - `NamlString` for heap-allocated strings with UTF-8 support
//! - `NamlStruct` for heap-allocated struct instances
//! - Exception handling primitives for try/catch support
//!
//! All heap objects use atomic reference counting for thread safety.
//! Values are passed as 64-bit tagged pointers or inline primitives.
//!

pub mod value;
pub mod exception;

pub use value::*;
pub use exception::*;
