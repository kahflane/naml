//!
//! Network Exception Types
//!
//! Provides exception types for network operations in naml.
//!
//! ## Exception Types
//!
//! - `NetworkError` - General network error with message and code
//! - `TimeoutError` - Operation timed out with message and timeout duration
//! - `ConnectionRefused` - Connection was refused by the remote host
//! - `DnsError` - DNS resolution failed for hostname
//! - `TlsError` - TLS/SSL error with message
//!
//! ## Exception Layout
//!
//! All exceptions follow the naml exception layout:
//! - Offset 0: message/first string field pointer (8 bytes)
//! - Offset 8: stack pointer (8 bytes) - captured at throw time
//! - Additional fields follow at offset 16+
//!

use naml_std_core::{NamlString, naml_exception_set, naml_stack_capture, naml_string_new};

/// Create a new NetworkError exception on the heap
///
/// Exception layout:
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes)
/// - Offset 16: code (8 bytes)
///
/// Total size: 24 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_network_error_new(message: *const NamlString, code: i64) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate NetworkError");
        }

        *(ptr as *mut i64) = message as i64;
        *(ptr.add(8) as *mut i64) = 0;
        *(ptr.add(16) as *mut i64) = code;

        ptr
    }
}

/// Create a new TimeoutError exception on the heap
///
/// Exception layout:
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes)
/// - Offset 16: timeout_ms (8 bytes)
///
/// Total size: 24 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_timeout_error_new(message: *const NamlString, timeout_ms: i64) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate TimeoutError");
        }

        *(ptr as *mut i64) = message as i64;
        *(ptr.add(8) as *mut i64) = 0;
        *(ptr.add(16) as *mut i64) = timeout_ms;

        ptr
    }
}

/// Create a new ConnectionRefused exception on the heap
///
/// Exception layout:
/// - Offset 0: address pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes)
///
/// Total size: 16 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_connection_refused_new(address: *const NamlString) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(16, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate ConnectionRefused");
        }

        *(ptr as *mut i64) = address as i64;
        *(ptr.add(8) as *mut i64) = 0;

        ptr
    }
}

/// Create a new DnsError exception on the heap
///
/// Exception layout:
/// - Offset 0: hostname pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes)
///
/// Total size: 16 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_dns_error_new(hostname: *const NamlString) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(16, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate DnsError");
        }

        *(ptr as *mut i64) = hostname as i64;
        *(ptr.add(8) as *mut i64) = 0;

        ptr
    }
}

/// Create a new TlsError exception on the heap
///
/// Exception layout:
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes)
///
/// Total size: 16 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_tls_error_new(message: *const NamlString) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(16, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate TlsError");
        }

        *(ptr as *mut i64) = message as i64;
        *(ptr.add(8) as *mut i64) = 0;

        ptr
    }
}

/// Helper to extract string from NamlString pointer
///
/// # Safety
/// The caller must ensure `s` is a valid pointer to a NamlString or null.
pub(crate) unsafe fn string_from_naml(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

/// Throw a NetworkError from a Rust std::io::Error
///
/// Sets the exception and returns null to indicate an exception was thrown.
pub(crate) fn throw_network_error(error: std::io::Error) -> *mut u8 {
    let code = error.raw_os_error().unwrap_or(-1) as i64;
    let message = error.to_string();

    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let net_error = naml_network_error_new(message_ptr, code);

        let stack = naml_stack_capture();
        *(net_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set(net_error);
    }

    std::ptr::null_mut()
}

/// Throw a TimeoutError
///
/// Sets the exception and returns null to indicate an exception was thrown.
pub(crate) fn throw_timeout_error(message: &str, timeout_ms: i64) -> *mut u8 {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let timeout_error = naml_timeout_error_new(message_ptr, timeout_ms);

        let stack = naml_stack_capture();
        *(timeout_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set(timeout_error);
    }

    std::ptr::null_mut()
}

/// Throw a ConnectionRefused error
///
/// Sets the exception and returns null to indicate an exception was thrown.
pub(crate) fn throw_connection_refused(address: &str) -> *mut u8 {
    unsafe {
        let address_ptr = naml_string_new(address.as_ptr(), address.len());
        let conn_error = naml_connection_refused_new(address_ptr);

        let stack = naml_stack_capture();
        *(conn_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set(conn_error);
    }

    std::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error_allocation() {
        unsafe {
            let msg = naml_string_new(b"connection failed".as_ptr(), 17);
            let error = naml_network_error_new(msg, 111);
            assert!(!error.is_null());

            let stored_msg = *(error as *const i64);
            assert_eq!(stored_msg, msg as i64);

            let stored_code = *(error.add(16) as *const i64);
            assert_eq!(stored_code, 111);

            std::alloc::dealloc(error, std::alloc::Layout::from_size_align(24, 8).unwrap());
        }
    }

    #[test]
    fn test_timeout_error_allocation() {
        unsafe {
            let msg = naml_string_new(b"operation timed out".as_ptr(), 19);
            let error = naml_timeout_error_new(msg, 5000);
            assert!(!error.is_null());

            let stored_timeout = *(error.add(16) as *const i64);
            assert_eq!(stored_timeout, 5000);

            std::alloc::dealloc(error, std::alloc::Layout::from_size_align(24, 8).unwrap());
        }
    }

    #[test]
    fn test_connection_refused_allocation() {
        unsafe {
            let addr = naml_string_new(b"127.0.0.1:8080".as_ptr(), 14);
            let error = naml_connection_refused_new(addr);
            assert!(!error.is_null());

            let stored_addr = *(error as *const i64);
            assert_eq!(stored_addr, addr as i64);

            std::alloc::dealloc(error, std::alloc::Layout::from_size_align(16, 8).unwrap());
        }
    }

    #[test]
    fn test_dns_error_allocation() {
        unsafe {
            let hostname = naml_string_new(b"example.invalid".as_ptr(), 15);
            let error = naml_dns_error_new(hostname);
            assert!(!error.is_null());

            let stored_hostname = *(error as *const i64);
            assert_eq!(stored_hostname, hostname as i64);

            std::alloc::dealloc(error, std::alloc::Layout::from_size_align(16, 8).unwrap());
        }
    }

    #[test]
    fn test_tls_error_allocation() {
        unsafe {
            let msg = naml_string_new(b"certificate expired".as_ptr(), 19);
            let error = naml_tls_error_new(msg);
            assert!(!error.is_null());

            let stored_msg = *(error as *const i64);
            assert_eq!(stored_msg, msg as i64);

            std::alloc::dealloc(error, std::alloc::Layout::from_size_align(16, 8).unwrap());
        }
    }
}
