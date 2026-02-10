///
/// TLS Client Implementation
///
/// Provides TLS client functions for naml programs. Wraps TCP connections
/// with TLS encryption using `rustls` and Mozilla root certificates.
///
/// ## Functions
///
/// - `naml_net_tls_client_connect` - Connect to a TLS server
/// - `naml_net_tls_client_read` - Read bytes from TLS socket
/// - `naml_net_tls_client_read_all` - Read all data until EOF
/// - `naml_net_tls_client_write` - Write bytes to TLS socket
/// - `naml_net_tls_client_close` - Close TLS socket
/// - `naml_net_tls_client_set_timeout` - Set read/write timeout
/// - `naml_net_tls_client_peer_addr` - Get peer address
///

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use naml_std_core::{NamlBytes, NamlString, naml_string_new};

use crate::errors::{string_from_naml, throw_network_error, throw_tls_error};
use crate::tcp::client::create_bytes_from;
use crate::tcp::server::next_handle;

use super::{TlsStream, get_tls_streams};

fn build_default_client_config() -> Arc<ClientConfig> {
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(
        rustls::crypto::ring::default_provider().into(),
    )
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(root_store)
    .with_no_client_auth();
    Arc::new(config)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tls_client_connect(address: *const NamlString) -> i64 {
    let addr_str = unsafe { string_from_naml(address) };

    let (hostname, _port) = match addr_str.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.to_string()),
        None => {
            throw_tls_error("invalid address: expected host:port");
            return -1;
        }
    };

    let server_name = match ServerName::try_from(hostname.clone()) {
        Ok(name) => name,
        Err(e) => {
            throw_tls_error(&format!("invalid hostname '{}': {}", hostname, e));
            return -1;
        }
    };

    let tcp_stream = match TcpStream::connect(&addr_str) {
        Ok(s) => s,
        Err(e) => {
            throw_network_error(e);
            return -1;
        }
    };

    let config = build_default_client_config();
    let conn = match ClientConnection::new(config, server_name) {
        Ok(c) => c,
        Err(e) => {
            throw_tls_error(&format!("TLS handshake failed: {}", e));
            return -1;
        }
    };

    let tls_stream = StreamOwned::new(conn, tcp_stream);
    let handle = next_handle();
    get_tls_streams()
        .lock()
        .unwrap()
        .insert(handle, TlsStream::Client(tls_stream));
    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_client_read(handle: i64, size: i64) -> *mut NamlBytes {
    let mut streams = get_tls_streams().lock().unwrap();
    let stream = match streams.get_mut(&handle) {
        Some(s) => s,
        None => {
            drop(streams);
            throw_network_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid TLS socket handle",
            ));
            return std::ptr::null_mut();
        }
    };

    let size = size.max(0) as usize;
    let mut buffer = vec![0u8; size];

    match stream.read(&mut buffer) {
        Ok(n) => create_bytes_from(buffer.as_ptr(), n),
        Err(e) => {
            drop(streams);
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_client_read_all(handle: i64) -> *mut NamlBytes {
    let mut streams = get_tls_streams().lock().unwrap();
    let stream = match streams.get_mut(&handle) {
        Some(s) => s,
        None => {
            drop(streams);
            throw_network_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid TLS socket handle",
            ));
            return std::ptr::null_mut();
        }
    };

    let mut buffer = Vec::new();

    match stream.read_to_end(&mut buffer) {
        Ok(_) => create_bytes_from(buffer.as_ptr(), buffer.len()),
        Err(e) => {
            drop(streams);
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tls_client_write(
    handle: i64,
    data: *const NamlBytes,
) -> i64 {
    if data.is_null() {
        return 0;
    }

    let mut streams = get_tls_streams().lock().unwrap();
    let stream = match streams.get_mut(&handle) {
        Some(s) => s,
        None => {
            drop(streams);
            throw_network_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid TLS socket handle",
            ));
            return -1;
        }
    };

    let len = unsafe { (*data).len };
    let bytes = unsafe { std::slice::from_raw_parts((*data).data.as_ptr(), len) };

    match stream.write_all(bytes) {
        Ok(()) => {
            let _ = stream.flush();
            len as i64
        }
        Err(e) => {
            drop(streams);
            throw_network_error(e);
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_client_close(handle: i64) {
    get_tls_streams().lock().unwrap().remove(&handle);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_client_set_timeout(handle: i64, ms: i64) {
    let streams = get_tls_streams().lock().unwrap();
    if let Some(stream) = streams.get(&handle) {
        let timeout = if ms <= 0 {
            None
        } else {
            Some(Duration::from_millis(ms as u64))
        };
        let tcp = stream.get_tcp_ref();
        let _ = tcp.set_read_timeout(timeout);
        let _ = tcp.set_write_timeout(timeout);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_client_peer_addr(handle: i64) -> *mut NamlString {
    let streams = get_tls_streams().lock().unwrap();
    match streams.get(&handle) {
        Some(stream) => {
            let addr = match stream.get_tcp_ref().peer_addr() {
                Ok(a) => a.to_string(),
                Err(_) => return std::ptr::null_mut(),
            };
            unsafe { naml_string_new(addr.as_ptr(), addr.len()) }
        }
        None => std::ptr::null_mut(),
    }
}
