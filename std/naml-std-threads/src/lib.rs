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
//! ## Mutex and RwLock
//!
//! Synchronization primitives for protecting shared state:
//! - `with_mutex<T>(value: T) -> mutex<T>` - Create a mutex with initial value
//! - `with_rwlock<T>(value: T) -> rwlock<T>` - Create a read-write lock
//! - `locked (val in mutex) { ... }` - Exclusive access block
//! - `rlocked (val in rwlock) { ... }` - Read access block
//! - `wlocked (val in rwlock) { ... }` - Write access block
//!
//! ## Platform Support
//!
//! Native platforms only. WASM targets use async/await instead of threads.
//!

pub mod scheduler;
pub mod channel;
pub mod mutex;
pub mod rwlock;

pub use scheduler::*;
pub use channel::*;
pub use mutex::*;
pub use rwlock::*;
