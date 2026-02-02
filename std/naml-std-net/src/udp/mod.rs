//!
//! UDP Networking Module
//!
//! Provides UDP socket operations for naml programs.
//!
//! ## Functions (std::net::udp)
//!
//! - `bind(address: string) -> udp_socket` - Bind a UDP socket
//! - `send(socket: udp_socket, data: bytes, address: string)` - Send data
//! - `receive(socket: udp_socket, size: int) -> bytes` - Receive data
//! - `receive_from(socket: udp_socket, size: int) -> udp_packet` - Receive with sender address
//! - `close(socket: udp_socket)` - Close socket
//!
//! ## Types
//!
//! ```naml
//! struct udp_packet {
//!     pub data: bytes,
//!     pub address: string
//! }
//! ```
//!

use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Mutex, OnceLock};

use std::alloc::Layout;
use naml_std_core::{naml_string_new, HeapHeader, HeapTag, NamlBytes, NamlString, NamlStruct};

use crate::errors::{string_from_naml, throw_network_error};

/// Create a NamlBytes from raw data
fn create_bytes_from(data: *const u8, len: usize) -> *mut NamlBytes {
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

/// Global registry for UDP sockets
static UDP_SOCKETS: OnceLock<Mutex<HashMap<i64, UdpSocket>>> = OnceLock::new();

/// Counter for generating unique handles
static UDP_HANDLE_COUNTER: OnceLock<Mutex<i64>> = OnceLock::new();

fn get_udp_sockets() -> &'static Mutex<HashMap<i64, UdpSocket>> {
    UDP_SOCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_udp_handle() -> i64 {
    let counter = UDP_HANDLE_COUNTER.get_or_init(|| Mutex::new(0));
    let mut guard = counter.lock().unwrap();
    *guard += 1;
    *guard
}

/// Bind a UDP socket to the given address
///
/// Returns a handle to the UDP socket, or -1 if an error occurred.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `address` - The address to bind to (e.g., "127.0.0.1:8080" or ":8080")
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_udp_bind(address: *const NamlString) -> i64 {
    let addr_str = unsafe { string_from_naml(address) };

    let bind_addr = if addr_str.starts_with(':') {
        format!("0.0.0.0{}", addr_str)
    } else {
        addr_str
    };

    match UdpSocket::bind(&bind_addr) {
        Ok(socket) => {
            let handle = next_udp_handle();
            get_udp_sockets().lock().unwrap().insert(handle, socket);
            handle
        }
        Err(e) => {
            throw_network_error(e);
            -1
        }
    }
}

/// Send data to a remote address
///
/// Returns the number of bytes sent, or -1 on error.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `socket_handle` - Handle to the UDP socket
/// * `data` - The data to send
/// * `address` - The destination address
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_udp_send(
    socket_handle: i64,
    data: *const NamlBytes,
    address: *const NamlString,
) -> i64 {
    if data.is_null() {
        return 0;
    }

    let addr_str = unsafe { string_from_naml(address) };
    let sockets = get_udp_sockets().lock().unwrap();

    let socket = match sockets.get(&socket_handle) {
        Some(s) => s,
        None => {
            let err = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid UDP socket handle",
            );
            drop(sockets);
            throw_network_error(err);
            return -1;
        }
    };

    let len = unsafe { (*data).len };
    let bytes = unsafe { std::slice::from_raw_parts((*data).data.as_ptr(), len) };

    match socket.send_to(bytes, &addr_str) {
        Ok(n) => n as i64,
        Err(e) => {
            drop(sockets);
            throw_network_error(e);
            -1
        }
    }
}

/// Receive data from the socket
///
/// Returns a pointer to NamlBytes containing the data, or null on error.
/// On error, a NetworkError exception is set.
///
/// # Arguments
/// * `socket_handle` - Handle to the UDP socket
/// * `size` - Maximum number of bytes to receive
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_udp_receive(socket_handle: i64, size: i64) -> *mut NamlBytes {
    // Clone the socket to avoid holding the lock during the blocking recv
    let socket_clone = {
        let sockets = get_udp_sockets().lock().unwrap();
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
                    "Invalid UDP socket handle",
                );
                drop(sockets);
                throw_network_error(err);
                return std::ptr::null_mut();
            }
        }
    };

    let size = size.max(0) as usize;
    let mut buffer = vec![0u8; size];

    match socket_clone.recv(&mut buffer) {
        Ok(n) => create_bytes_from(buffer.as_ptr(), n),
        Err(e) => {
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

/// Receive data from the socket with sender address
///
/// Returns a pointer to NamlStruct (udp_packet) containing data and address,
/// or null on error. On error, a NetworkError exception is set.
///
/// The udp_packet struct has:
/// - field 0: data (NamlArray of bytes)
/// - field 1: address (NamlString)
///
/// # Arguments
/// * `socket_handle` - Handle to the UDP socket
/// * `size` - Maximum number of bytes to receive
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_udp_receive_from(socket_handle: i64, size: i64) -> *mut NamlStruct {
    // Clone the socket to avoid holding the lock during the blocking recv_from
    let socket_clone = {
        let sockets = get_udp_sockets().lock().unwrap();
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
                    "Invalid UDP socket handle",
                );
                drop(sockets);
                throw_network_error(err);
                return std::ptr::null_mut();
            }
        }
    };

    let size = size.max(0) as usize;
    let mut buffer = vec![0u8; size];

    match socket_clone.recv_from(&mut buffer) {
        Ok((n, addr)) => {
            unsafe {
                // Create data array
                let data_arr = naml_std_core::naml_array_new(n);
                for &byte in buffer[..n].iter() {
                    naml_std_core::naml_array_push(data_arr, byte as i64);
                }

                // Create address string
                let addr_str = addr.to_string();
                let addr_ptr = naml_string_new(addr_str.as_ptr(), addr_str.len());

                // Create udp_packet struct with 2 fields
                // Type ID 0 is used for anonymous/runtime structs
                let packet = naml_std_core::naml_struct_new(0, 2);
                naml_std_core::naml_struct_set_field(packet, 0, data_arr as i64);
                naml_std_core::naml_struct_set_field(packet, 1, addr_ptr as i64);

                packet
            }
        }
        Err(e) => {
            throw_network_error(e);
            std::ptr::null_mut()
        }
    }
}

/// Close a UDP socket
///
/// # Arguments
/// * `socket_handle` - Handle to the UDP socket to close
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_udp_close(socket_handle: i64) {
    get_udp_sockets().lock().unwrap().remove(&socket_handle);
}

/// Get the local address of a UDP socket
///
/// Returns the address as a string, or null if an error occurred.
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_udp_local_addr(socket_handle: i64) -> *mut NamlString {
    let sockets = get_udp_sockets().lock().unwrap();

    let socket = match sockets.get(&socket_handle) {
        Some(s) => s,
        None => {
            return std::ptr::null_mut();
        }
    };

    match socket.local_addr() {
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
    use naml_std_core::naml_string_new;
    use std::alloc::Layout;
    use naml_std_core::{HeapHeader, HeapTag};

    /// Create a NamlBytes from raw data for testing
    fn create_bytes_from(data: *const u8, len: usize) -> *mut NamlBytes {
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
    fn test_bind_and_close() {
        unsafe {
            let addr = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let handle = naml_net_udp_bind(addr);
            assert!(handle > 0, "Failed to bind UDP socket");

            let local_addr = naml_net_udp_local_addr(handle);
            assert!(!local_addr.is_null(), "Failed to get local address");

            naml_net_udp_close(handle);

            // Verify socket was removed
            let sockets = get_udp_sockets().lock().unwrap();
            assert!(!sockets.contains_key(&handle));
        }
    }

    #[test]
    fn test_send_and_receive() {
        unsafe {
            // Create two sockets
            let addr1 = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let socket1 = naml_net_udp_bind(addr1);
            assert!(socket1 > 0);

            let addr2 = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let socket2 = naml_net_udp_bind(addr2);
            assert!(socket2 > 0);

            // Get socket2's address
            let socket2_addr = naml_net_udp_local_addr(socket2);
            let socket2_addr_str = string_from_naml(socket2_addr);

            // Send from socket1 to socket2
            let message = create_bytes_from(b"Hello UDP!".as_ptr(), 10);
            let dest_addr = naml_string_new(socket2_addr_str.as_ptr(), socket2_addr_str.len());
            let sent = naml_net_udp_send(socket1, message, dest_addr);
            assert_eq!(sent, 10);

            // Receive on socket2
            let received = naml_net_udp_receive(socket2, 1024);
            assert!(!received.is_null());

            let data = array_to_bytes(received);
            assert_eq!(data, b"Hello UDP!");

            naml_net_udp_close(socket1);
            naml_net_udp_close(socket2);
        }
    }

    #[test]
    fn test_receive_from() {
        unsafe {
            // Create two sockets
            let addr1 = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let socket1 = naml_net_udp_bind(addr1);
            assert!(socket1 > 0);

            let addr2 = naml_string_new(b"127.0.0.1:0".as_ptr(), 11);
            let socket2 = naml_net_udp_bind(addr2);
            assert!(socket2 > 0);

            // Get addresses
            let socket1_addr = naml_net_udp_local_addr(socket1);
            let socket1_addr_str = string_from_naml(socket1_addr);
            let socket2_addr = naml_net_udp_local_addr(socket2);
            let socket2_addr_str = string_from_naml(socket2_addr);

            // Send from socket1 to socket2
            let message = create_bytes_from(b"Test packet".as_ptr(), 11);
            let dest_addr = naml_string_new(socket2_addr_str.as_ptr(), socket2_addr_str.len());
            let sent = naml_net_udp_send(socket1, message, dest_addr);
            assert_eq!(sent, 11);

            // Receive with address on socket2
            let packet = naml_net_udp_receive_from(socket2, 1024);
            assert!(!packet.is_null());

            // Extract data from packet (field 0)
            let data_arr = naml_std_core::naml_struct_get_field(packet, 0) as *const NamlArray;
            let data = array_to_bytes(data_arr);
            assert_eq!(data, b"Test packet");

            // Extract sender address from packet (field 1)
            let sender_addr = naml_std_core::naml_struct_get_field(packet, 1) as *const NamlString;
            let sender_addr_str = string_from_naml(sender_addr);
            assert_eq!(sender_addr_str, socket1_addr_str);

            naml_net_udp_close(socket1);
            naml_net_udp_close(socket2);
        }
    }

    #[test]
    fn test_bind_invalid_address() {
        unsafe {
            let addr = naml_string_new(b"invalid:address".as_ptr(), 15);
            let handle = naml_net_udp_bind(addr);
            assert_eq!(handle, -1, "Should fail with invalid address");
        }
    }

    #[test]
    fn test_send_invalid_handle() {
        unsafe {
            let data = create_bytes_from(b"test".as_ptr(), 4);
            let addr = naml_string_new(b"127.0.0.1:1234".as_ptr(), 14);
            let result = naml_net_udp_send(99999, data, addr);
            assert_eq!(result, -1, "Should fail with invalid socket handle");
        }
    }

    #[test]
    fn test_receive_invalid_handle() {
        let result = naml_net_udp_receive(99999, 1024);
        assert!(result.is_null(), "Should fail with invalid socket handle");
    }
}
