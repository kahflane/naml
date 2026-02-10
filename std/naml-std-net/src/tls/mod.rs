///
/// TLS/SSL Socket Implementation
///
/// Provides TLS-wrapped TCP sockets for encrypted network communication.
/// Uses `rustls` (pure Rust, no OpenSSL dependency).
///
/// ## Client Functions
///
/// - `naml_net_tls_client_connect` - Connect to a TLS server (uses system root CAs)
/// - `naml_net_tls_client_read` - Read bytes from TLS socket
/// - `naml_net_tls_client_read_all` - Read all data until EOF
/// - `naml_net_tls_client_write` - Write bytes to TLS socket
/// - `naml_net_tls_client_close` - Close TLS socket
/// - `naml_net_tls_client_set_timeout` - Set read/write timeout
/// - `naml_net_tls_client_peer_addr` - Get peer address
///
/// ## Server Functions
///
/// - `naml_net_tls_server_wrap_listener` - Wrap TCP listener with TLS config
/// - `naml_net_tls_server_accept` - Accept a TLS connection
/// - `naml_net_tls_server_close_listener` - Close TLS listener
///
/// ## Handle Management
///
/// TLS sockets are stored in a global registry and accessed via integer handles,
/// following the same pattern as TCP sockets. TLS streams cannot be cloned, so
/// the mutex is held during blocking I/O operations.
///

pub mod client;
pub mod server;

pub use client::*;
pub use server::*;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock};

use rustls::{ClientConnection, ServerConnection, StreamOwned};

pub enum TlsStream {
    Client(StreamOwned<ClientConnection, TcpStream>),
    Server(StreamOwned<ServerConnection, TcpStream>),
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            TlsStream::Client(s) => s.read(buf),
            TlsStream::Server(s) => s.read(buf),
        }
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            TlsStream::Client(s) => s.write(buf),
            TlsStream::Server(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            TlsStream::Client(s) => s.flush(),
            TlsStream::Server(s) => s.flush(),
        }
    }
}

impl TlsStream {
    pub fn get_tcp_ref(&self) -> &TcpStream {
        match self {
            TlsStream::Client(s) => s.get_ref(),
            TlsStream::Server(s) => s.get_ref(),
        }
    }
}

static TLS_STREAMS: OnceLock<Mutex<HashMap<i64, TlsStream>>> = OnceLock::new();

pub(crate) fn get_tls_streams() -> &'static Mutex<HashMap<i64, TlsStream>> {
    TLS_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}
