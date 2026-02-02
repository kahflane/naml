//!
//! TCP Server Implementation
//!
//! Provides TCP server functions for naml programs.
//!
//! ## Functions
//!
//! - `naml_net_tcp_server_listen` - Bind and listen on an address
//! - `naml_net_tcp_server_accept` - Accept an incoming connection
//!
//! ## Handle Management
//!
//! TCP listeners and sockets are stored in a global registry and accessed
//! via integer handles. This allows the naml runtime to manage resources
//! without exposing Rust types directly.
//!

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};

use naml_std_core::{naml_string_new, NamlString};

use crate::errors::{string_from_naml, throw_network_error};

/// Global registry for TCP listeners
static LISTENERS: OnceLock<Mutex<HashMap<i64, TcpListener>>> = OnceLock::new();

/// Global registry for TCP sockets (streams)
static SOCKETS: OnceLock<Mutex<HashMap<i64, TcpStream>>> = OnceLock::new();

/// Counter for generating unique handles
static HANDLE_COUNTER: OnceLock<Mutex<i64>> = OnceLock::new();

fn get_listeners() -> &'static Mutex<HashMap<i64, TcpListener>> {
    LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn get_sockets() -> &'static Mutex<HashMap<i64, TcpStream>> {
    SOCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn next_handle() -> i64 {
    let counter = HANDLE_COUNTER.get_or_init(|| Mutex::new(0));
    let mut guard = counter.lock().unwrap();
    *guard += 1;
    *guard
}

/// Bind and listen on the given address
///
/// Returns a handle to the TCP listener, or -1 if an error occurred.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `address` - The address to bind to (e.g., "127.0.0.1:8080" or ":8080")
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tcp_server_listen(address: *const NamlString) -> i64 {
    let addr_str = unsafe { string_from_naml(address) };

    let bind_addr = if addr_str.starts_with(':') {
        format!("0.0.0.0{}", addr_str)
    } else {
        addr_str
    };

    match TcpListener::bind(&bind_addr) {
        Ok(listener) => {
            let handle = next_handle();
            get_listeners().lock().unwrap().insert(handle, listener);
            handle
        }
        Err(e) => {
            throw_network_error(e);
            -1
        }
    }
}

/// Accept an incoming connection on the listener
///
/// Returns a handle to the TCP socket, or -1 if an error occurred.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `listener_handle` - Handle to the TCP listener from `listen()`
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_server_accept(listener_handle: i64) -> i64 {
    // Clone the listener to avoid holding the lock during the blocking accept
    let listener_clone = {
        let listeners = get_listeners().lock().unwrap();
        match listeners.get(&listener_handle) {
            Some(l) => match l.try_clone() {
                Ok(cloned) => cloned,
                Err(e) => {
                    drop(listeners);
                    throw_network_error(e);
                    return -1;
                }
            },
            None => {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Invalid listener handle",
                );
                drop(listeners);
                throw_network_error(err);
                return -1;
            }
        }
    };

    match listener_clone.accept() {
        Ok((stream, _addr)) => {
            let handle = next_handle();
            get_sockets().lock().unwrap().insert(handle, stream);
            handle
        }
        Err(e) => {
            throw_network_error(e);
            -1
        }
    }
}

/// Get the local address of a listener
///
/// Returns the address as a string, or null if an error occurred.
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_server_local_addr(listener_handle: i64) -> *mut NamlString {
    let listeners = get_listeners().lock().unwrap();

    let listener = match listeners.get(&listener_handle) {
        Some(l) => l,
        None => {
            return std::ptr::null_mut();
        }
    };

    match listener.local_addr() {
        Ok(addr) => {
            let addr_str = addr.to_string();
            unsafe { naml_string_new(addr_str.as_ptr(), addr_str.len()) }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Close a TCP listener
///
/// # Arguments
/// * `listener_handle` - Handle to the TCP listener to close
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_server_close(listener_handle: i64) {
    get_listeners().lock().unwrap().remove(&listener_handle);
}

/// Get the remote address of a connected socket
///
/// Returns the address as a string, or null if an error occurred.
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_socket_peer_addr(socket_handle: i64) -> *mut NamlString {
    let sockets = get_sockets().lock().unwrap();

    let socket = match sockets.get(&socket_handle) {
        Some(s) => s,
        None => {
            return std::ptr::null_mut();
        }
    };

    match socket.peer_addr() {
        Ok(addr) => {
            let addr_str = addr.to_string();
            unsafe { naml_string_new(addr_str.as_ptr(), addr_str.len()) }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_listen_and_accept() {
        unsafe {
            let addr = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let listener_handle = naml_net_tcp_server_listen(addr);
            assert!(listener_handle > 0, "Failed to create listener");

            let local_addr = naml_net_tcp_server_local_addr(listener_handle);
            assert!(!local_addr.is_null(), "Failed to get local address");

            let addr_str = string_from_naml(local_addr);
            assert!(addr_str.contains("127.0.0.1:"), "Invalid local address format");

            let port: u16 = addr_str.split(':').last().unwrap().parse().unwrap();

            let client_thread = thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                let _ = TcpStream::connect(format!("127.0.0.1:{}", port));
            });

            let socket_handle = naml_net_tcp_server_accept(listener_handle);
            assert!(socket_handle > 0, "Failed to accept connection");

            let peer_addr = naml_net_tcp_socket_peer_addr(socket_handle);
            assert!(!peer_addr.is_null(), "Failed to get peer address");

            client_thread.join().unwrap();

            naml_net_tcp_server_close(listener_handle);

            let listeners = get_listeners().lock().unwrap();
            assert!(!listeners.contains_key(&listener_handle));
        }
    }

    #[test]
    fn test_listen_invalid_address() {
        unsafe {
            let addr = naml_string_new(b"invalid:address:format".as_ptr(), 22);
            let handle = naml_net_tcp_server_listen(addr);
            assert_eq!(handle, -1, "Should fail with invalid address");
        }
    }

    #[test]
    fn test_accept_invalid_handle() {
        let handle = naml_net_tcp_server_accept(99999);
        assert_eq!(handle, -1, "Should fail with invalid listener handle");
    }
}
