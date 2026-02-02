//!
//! TCP Client Implementation
//!
//! Provides TCP client functions for naml programs.
//!
//! ## Functions
//!
//! - `naml_net_tcp_client_connect` - Connect to a remote server
//! - `naml_net_tcp_client_read` - Read specified bytes from socket
//! - `naml_net_tcp_client_read_all` - Read all available data from socket
//! - `naml_net_tcp_client_write` - Write data to socket
//! - `naml_net_tcp_client_close` - Close the socket
//! - `naml_net_tcp_client_set_timeout` - Set read/write timeout
//!

use std::alloc::Layout;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use naml_std_core::{HeapHeader, HeapTag, NamlArray, NamlBytes, NamlString};

use crate::errors::{string_from_naml, throw_connection_refused, throw_network_error};

use super::server::{get_sockets, next_handle};

/// Create a NamlBytes from raw data
pub(crate) fn create_bytes_from(data: *const u8, len: usize) -> *mut NamlBytes {
    unsafe {
        let cap = if len == 0 { 8 } else { len };
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        )
        .unwrap();
        let ptr = std::alloc::alloc(layout) as *mut NamlBytes;
        (*ptr).header = HeapHeader::new(HeapTag::Bytes);
        (*ptr).len = len;
        (*ptr).capacity = cap;
        if len > 0 && !data.is_null() {
            std::ptr::copy_nonoverlapping(data, (*ptr).data.as_mut_ptr(), len);
        }
        ptr
    }
}

/// Connect to a remote server
///
/// Returns a handle to the TCP socket, or -1 if an error occurred.
/// On error, a NetworkError or ConnectionRefused exception is set.
///
/// # Arguments
/// * `address` - The address to connect to (e.g., "127.0.0.1:8080")
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tcp_client_connect(address: *const NamlString) -> i64 {
    let addr_str = unsafe { string_from_naml(address) };

    match TcpStream::connect(&addr_str) {
        Ok(stream) => {
            let handle = next_handle();
            get_sockets().lock().unwrap().insert(handle, stream);
            handle
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                throw_connection_refused(&addr_str);
            } else {
                throw_network_error(e);
            }
            -1
        }
    }
}

/// Read up to `size` bytes from the socket
///
/// Returns a pointer to NamlBytes containing the data read, or null on error.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `socket_handle` - Handle to the TCP socket
/// * `size` - Maximum number of bytes to read
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_client_read(socket_handle: i64, size: i64) -> *mut NamlBytes {
    // Clone the stream to avoid holding the lock during the blocking read
    let mut stream_clone = {
        let sockets = get_sockets().lock().unwrap();
        match sockets.get(&socket_handle) {
            Some(s) => match s.try_clone() {
                Ok(cloned) => cloned,
                Err(e) => {
                    drop(sockets);
                    throw_network_error(e);
                    return std::ptr::null_mut();
                }
            },
            None => {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Invalid socket handle",
                );
                drop(sockets);
                throw_network_error(err);
                return std::ptr::null_mut();
            }
        }
    };

    let size = size.max(0) as usize;
    let mut buffer = vec![0u8; size];

    match stream_clone.read(&mut buffer) {
        Ok(n) => create_bytes_from(buffer.as_ptr(), n),
        Err(e) => {
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

/// Read all available data from the socket until EOF
///
/// Returns a pointer to NamlBytes containing all data read, or null on error.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `socket_handle` - Handle to the TCP socket
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_client_read_all(socket_handle: i64) -> *mut NamlBytes {
    // Clone the stream to avoid holding the lock during the blocking read
    let mut stream_clone = {
        let sockets = get_sockets().lock().unwrap();
        match sockets.get(&socket_handle) {
            Some(s) => match s.try_clone() {
                Ok(cloned) => cloned,
                Err(e) => {
                    drop(sockets);
                    throw_network_error(e);
                    return std::ptr::null_mut();
                }
            },
            None => {
                let err = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Invalid socket handle",
                );
                drop(sockets);
                throw_network_error(err);
                return std::ptr::null_mut();
            }
        }
    };

    let mut buffer = Vec::new();

    match stream_clone.read_to_end(&mut buffer) {
        Ok(_) => create_bytes_from(buffer.as_ptr(), buffer.len()),
        Err(e) => {
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

/// Write data to the socket
///
/// Returns the number of bytes written, or -1 on error.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `socket_handle` - Handle to the TCP socket
/// * `data` - The data to write
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tcp_client_write(
    socket_handle: i64,
    data: *const NamlBytes,
) -> i64 {
    if data.is_null() {
        return 0;
    }

    let mut sockets = get_sockets().lock().unwrap();

    let stream = match sockets.get_mut(&socket_handle) {
        Some(s) => s,
        None => {
            let err = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid socket handle",
            );
            drop(sockets);
            throw_network_error(err);
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
            drop(sockets);
            throw_network_error(e);
            -1
        }
    }
}

/// Close the TCP socket
///
/// # Arguments
/// * `socket_handle` - Handle to the TCP socket to close
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_client_close(socket_handle: i64) {
    get_sockets().lock().unwrap().remove(&socket_handle);
}

/// Set read/write timeout on the socket
///
/// # Arguments
/// * `socket_handle` - Handle to the TCP socket
/// * `ms` - Timeout in milliseconds (0 to disable timeout)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tcp_client_set_timeout(socket_handle: i64, ms: i64) {
    let sockets = get_sockets().lock().unwrap();

    if let Some(stream) = sockets.get(&socket_handle) {
        let timeout = if ms <= 0 {
            None
        } else {
            Some(Duration::from_millis(ms as u64))
        };

        let _ = stream.set_read_timeout(timeout);
        let _ = stream.set_write_timeout(timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp::server::naml_net_tcp_server_listen;
    use naml_std_core::naml_string_new;
    use std::thread;
    use std::time::Duration;

    /// Helper to convert NamlArray of bytes to Vec<u8>
    unsafe fn array_to_bytes(arr: *const NamlArray) -> Vec<u8> {
        unsafe {
            let len = naml_std_core::naml_array_len(arr) as usize;
            let mut bytes = Vec::with_capacity(len);
            for i in 0..len {
                bytes.push(naml_std_core::naml_array_get(arr, i as i64) as u8);
            }
            bytes
        }
    }

    #[test]
    fn test_connect_and_communicate() {
        unsafe {
            let server_addr = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let listener_handle = naml_net_tcp_server_listen(server_addr);
            assert!(listener_handle > 0, "Failed to create listener");

            let local_addr = crate::tcp::server::naml_net_tcp_server_local_addr(listener_handle);
            let addr_str = string_from_naml(local_addr);
            let port: u16 = addr_str.split(':').last().unwrap().parse().unwrap();

            let server_thread = thread::spawn(move || {
                let client_socket = crate::tcp::server::naml_net_tcp_server_accept(listener_handle);
                assert!(client_socket > 0);

                let data = naml_net_tcp_client_read(client_socket, 1024);
                assert!(!data.is_null());

                let received = array_to_bytes(data);
                assert_eq!(received, b"Hello, server!");

                let response = create_bytes_from(b"Hello, client!".as_ptr(), 14);
                let written = naml_net_tcp_client_write(client_socket, response);
                assert_eq!(written, 14);

                naml_net_tcp_client_close(client_socket);
            });

            thread::sleep(Duration::from_millis(50));

            let connect_addr = format!("127.0.0.1:{}", port);
            let connect_addr_ptr = naml_string_new(connect_addr.as_ptr(), connect_addr.len());
            let client_socket = naml_net_tcp_client_connect(connect_addr_ptr);
            assert!(client_socket > 0, "Failed to connect");

            let message = create_bytes_from(b"Hello, server!".as_ptr(), 14);
            let written = naml_net_tcp_client_write(client_socket, message);
            assert_eq!(written, 14);

            thread::sleep(Duration::from_millis(50));

            let response = naml_net_tcp_client_read(client_socket, 1024);
            assert!(!response.is_null());

            let received = array_to_bytes(response);
            assert_eq!(received, b"Hello, client!");

            naml_net_tcp_client_close(client_socket);
            server_thread.join().unwrap();

            crate::tcp::server::naml_net_tcp_server_close(listener_handle);
        }
    }

    #[test]
    fn test_connect_refused() {
        unsafe {
            let addr = naml_string_new(b"127.0.0.1:1".as_ptr(), 11);
            let handle = naml_net_tcp_client_connect(addr);
            assert_eq!(handle, -1, "Should fail to connect to closed port");
        }
    }

    #[test]
    fn test_read_invalid_handle() {
        let result = naml_net_tcp_client_read(99999, 1024);
        assert!(result.is_null(), "Should fail with invalid socket handle");
    }

    #[test]
    fn test_write_invalid_handle() {
        unsafe {
            let data = create_bytes_from(b"test".as_ptr(), 4);
            let result = naml_net_tcp_client_write(99999, data);
            assert_eq!(result, -1, "Should fail with invalid socket handle");
        }
    }

    #[test]
    fn test_set_timeout() {
        unsafe {
            let server_addr = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let listener_handle = naml_net_tcp_server_listen(server_addr);

            let local_addr = crate::tcp::server::naml_net_tcp_server_local_addr(listener_handle);
            let addr_str = string_from_naml(local_addr);
            let port: u16 = addr_str.split(':').last().unwrap().parse().unwrap();

            let _server_thread = thread::spawn(move || {
                let _ = crate::tcp::server::naml_net_tcp_server_accept(listener_handle);
                thread::sleep(Duration::from_millis(200));
            });

            thread::sleep(Duration::from_millis(50));

            let connect_addr = format!("127.0.0.1:{}", port);
            let connect_addr_ptr = naml_string_new(connect_addr.as_ptr(), connect_addr.len());
            let client_socket = naml_net_tcp_client_connect(connect_addr_ptr);
            assert!(client_socket > 0);

            naml_net_tcp_client_set_timeout(client_socket, 100);

            naml_net_tcp_client_close(client_socket);
            crate::tcp::server::naml_net_tcp_server_close(listener_handle);
        }
    }
}
