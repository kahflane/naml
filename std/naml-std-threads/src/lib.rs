//!
//! naml-std-threads - Concurrency Primitives
//!
//! Provides threading and communication primitives for naml programs.
//!
//! ## Task Scheduling
//!
//! Implements an M:N threading model where M user-space tasks are multiplexed
//! onto N OS threads. Features:
//! - Thread pool with configurable worker count (defaults to CPU cores)
//! - Work-stealing queue for load balancing
//! - Closure support for captured variables
//!
//! ## Channels
//!
//! Bounded channels for inter-task communication:
//! - `open_channel<T>(capacity: int) -> channel<T>` - Create a bounded channel
//! - `channel.send(value)` - Send value (blocks if full)
//! - `channel.receive() -> T` - Receive value (blocks if empty)
//! - `channel.close()` - Close the channel
//!
//! ## Platform Support
//!
//! Native platforms only. WASM targets use async/await instead of threads.
//!

pub mod scheduler;
pub mod channel;

pub use scheduler::*;
pub use channel::*;
