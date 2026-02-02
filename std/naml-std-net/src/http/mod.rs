//!
//! HTTP Module
//!
//! Provides HTTP client and server capabilities for naml programs.
//!
//! ## Submodules
//!
//! - `types` - Core HTTP types (request, response)
//!
//! ## Types
//!
//! ### request
//! ```naml
//! struct request {
//!     pub method: string,
//!     pub path: string,
//!     pub headers: map<string, string>,
//!     pub body: bytes,
//!     pub params: map<string, string>,
//!     pub query: map<string, string>
//! }
//! ```
//!
//! ### response
//! ```naml
//! struct response {
//!     pub status: int,
//!     pub headers: map<string, string>,
//!     pub body: bytes
//! }
//! ```
//!
//! ## Handler and Middleware Types
//!
//! ```naml
//! pub type handler = fn(request) -> response;
//! pub type middleware = fn(handler) -> handler;
//! ```
//!

pub mod client;
pub mod middleware;
pub mod server;
pub mod types;

pub use client::*;
pub use middleware::*;
pub use server::*;
pub use types::*;
