//!
//! naml-std-net - Networking Operations
//!
//! Provides networking capabilities for naml programs including TCP, UDP,
//! HTTP client/server, and middleware support.
//!
//! ## Module Structure
//!
//! - `std::net::tcp::server` - TCP server (listen, accept)
//! - `std::net::tcp::client` - TCP client (connect, read, write, close)
//! - `std::net::udp` - UDP socket operations
//! - `std::net::http::client` - HTTP client (get, post, put, patch, delete)
//! - `std::net::http::server` - HTTP server with chi-style routing
//! - `std::net::http::middleware` - Built-in middleware
//!
//! ## TCP Server API (std::net::tcp::server)
//!
//! - `listen(address: string) -> tcp_listener throws NetworkError`
//! - `accept(listener: tcp_listener) -> tcp_socket throws NetworkError`
//!
//! ## TCP Client API (std::net::tcp::client)
//!
//! - `connect(address: string) -> tcp_socket throws NetworkError, TimeoutError`
//! - `read(socket: tcp_socket, size: int) -> bytes throws NetworkError`
//! - `read_all(socket: tcp_socket) -> bytes throws NetworkError`
//! - `write(socket: tcp_socket, data: bytes) throws NetworkError`
//! - `close(socket: tcp_socket)`
//! - `set_timeout(socket: tcp_socket, ms: int)`
//!
//! ## UDP API (std::net::udp)
//!
//! - `bind(address: string) -> udp_socket throws NetworkError`
//! - `send(socket: udp_socket, data: bytes, address: string) throws NetworkError`
//! - `receive(socket: udp_socket, size: int) -> bytes throws NetworkError`
//! - `receive_from(socket: udp_socket, size: int) -> udp_packet throws NetworkError`
//! - `close(socket: udp_socket)`
//!
//! ## HTTP Client API (std::net::http::client)
//!
//! - `get(url: string) -> response throws NetworkError, TimeoutError`
//! - `post(url: string, body: bytes) -> response throws NetworkError, TimeoutError`
//! - `put(url: string, body: bytes) -> response throws NetworkError, TimeoutError`
//! - `patch(url: string, body: bytes) -> response throws NetworkError, TimeoutError`
//! - `delete(url: string) -> response throws NetworkError, TimeoutError`
//!
//! ## HTTP Server API (std::net::http::server)
//!
//! - `open_router() -> router`
//! - `get(r: router, pattern: string, h: handler)`
//! - `post(r: router, pattern: string, h: handler)`
//! - `with(r: router, mw: middleware)`
//! - `serve(address: string, r: router) throws NetworkError`
//!
//! ## Middleware API (std::net::http::middleware)
//!
//! - `logger() -> middleware`
//! - `timeout(ms: int) -> middleware`
//! - `recover() -> middleware`
//! - `cors(origins: [string]) -> middleware`
//! - `rate_limit(requests_per_second: int) -> middleware`
//! - `compress() -> middleware`
//! - `request_id() -> middleware`
//!
//! ## Exceptions
//!
//! - `NetworkError { message: string, code: int }` - General network error
//! - `TimeoutError { message: string, timeout_ms: int }` - Operation timed out
//! - `ConnectionRefused { address: string }` - Connection refused by remote host
//! - `DnsError { hostname: string }` - DNS resolution failed
//! - `TlsError { message: string }` - TLS/SSL error
//!
//! ## Platform Support
//!
//! Native platform first. Server WASM and Browser WASM support planned.
//!

mod errors;
pub mod http;
pub mod tcp;
pub mod tls;
pub mod udp;

pub use errors::*;
pub use http::*;
pub use tcp::*;
pub use tls::*;
pub use udp::*;
