//!
//! TCP Networking Module
//!
//! Provides TCP client and server operations for naml programs.
//!
//! ## Server Functions (std::net::tcp::server)
//!
//! - `listen(addr: string) -> tcp_listener` - Start a TCP listener
//! - `accept(listener: tcp_listener) -> tcp_socket` - Accept connection
//!
//! ## Client Functions (std::net::tcp::client)
//!
//! - `connect(addr: string) -> tcp_socket` - Connect to server
//! - `read(socket: tcp_socket, size: int) -> bytes` - Read data from socket
//! - `read_all(socket: tcp_socket) -> bytes` - Read all available data
//! - `write(socket: tcp_socket, data: bytes)` - Write data to socket
//! - `close(socket: tcp_socket)` - Close socket
//! - `set_timeout(socket: tcp_socket, ms: int)` - Set socket timeout
//!

pub(crate) mod client;
pub(crate) mod server;

pub use client::*;
pub use server::*;
