//!
//! HTTP Core Types
//!
//! Provides the core types for HTTP operations in naml.
//!
//! ## Struct Types
//!
//! ### request
//! ```naml
//! struct request {
//!     pub method: string,      // HTTP method (GET, POST, etc.)
//!     pub path: string,        // Request path
//!     pub headers: map<string, string>,
//!     pub body: bytes,
//!     pub params: map<string, string>,  // URL path parameters
//!     pub query: map<string, string>    // Query string parameters
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
//! ## Type IDs
//!
//! - Request: TYPE_ID_REQUEST (1001)
//! - Response: TYPE_ID_RESPONSE (1002)
//!

use std::alloc::Layout;

use naml_std_core::{
    naml_string_new, HeapHeader, HeapTag, NamlArray, NamlBytes, NamlString, NamlStruct,
};

/// Type ID for HTTP request struct
pub const TYPE_ID_REQUEST: u32 = 1001;

/// Type ID for HTTP response struct
pub const TYPE_ID_RESPONSE: u32 = 1002;

/// Request field indices
pub mod request_fields {
    pub const METHOD: u32 = 0;
    pub const PATH: u32 = 1;
    pub const HEADERS: u32 = 2;
    pub const BODY: u32 = 3;
    pub const PARAMS: u32 = 4;
    pub const QUERY: u32 = 5;
    pub const FIELD_COUNT: u32 = 6;
}

/// Response field indices
pub mod response_fields {
    pub const STATUS: u32 = 0;
    pub const HEADERS: u32 = 1;
    pub const BODY: u32 = 2;
    pub const FIELD_COUNT: u32 = 3;
}

/// Create a new HTTP request struct
///
/// Fields are initialized to:
/// - method: empty string
/// - path: empty string
/// - headers: null (caller should set)
/// - body: empty array
/// - params: null (caller should set)
/// - query: null (caller should set)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_request_new() -> *mut NamlStruct {
    unsafe {
        let req = naml_std_core::naml_struct_new(TYPE_ID_REQUEST, request_fields::FIELD_COUNT);

        // Initialize method to empty string
        let method = naml_string_new(std::ptr::null(), 0);
        naml_std_core::naml_struct_set_field(req, request_fields::METHOD, method as i64);

        // Initialize path to empty string
        let path = naml_string_new(std::ptr::null(), 0);
        naml_std_core::naml_struct_set_field(req, request_fields::PATH, path as i64);

        // Headers, params, query are set to null - caller provides maps
        naml_std_core::naml_struct_set_field(req, request_fields::HEADERS, 0);
        naml_std_core::naml_struct_set_field(req, request_fields::PARAMS, 0);
        naml_std_core::naml_struct_set_field(req, request_fields::QUERY, 0);

        // Initialize body to empty array
        let body = naml_std_core::naml_array_new(0);
        naml_std_core::naml_struct_set_field(req, request_fields::BODY, body as i64);

        req
    }
}

/// Create a new HTTP response struct
///
/// Fields are initialized to:
/// - status: 200
/// - headers: null (caller should set)
/// - body: empty array
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_response_new() -> *mut NamlStruct {
    unsafe {
        let res = naml_std_core::naml_struct_new(TYPE_ID_RESPONSE, response_fields::FIELD_COUNT);

        // Initialize status to 200
        naml_std_core::naml_struct_set_field(res, response_fields::STATUS, 200);

        // Headers set to null - caller provides map
        naml_std_core::naml_struct_set_field(res, response_fields::HEADERS, 0);

        // Initialize body to empty array
        let body = naml_std_core::naml_array_new(0);
        naml_std_core::naml_struct_set_field(res, response_fields::BODY, body as i64);

        res
    }
}

/// Create a response with status, headers, and body
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_create(
    status: i64,
    headers: i64,
    body: *mut NamlArray,
) -> *mut NamlStruct {
    unsafe {
        let res = naml_std_core::naml_struct_new(TYPE_ID_RESPONSE, response_fields::FIELD_COUNT);

        naml_std_core::naml_struct_set_field(res, response_fields::STATUS, status);
        naml_std_core::naml_struct_set_field(res, response_fields::HEADERS, headers);
        naml_std_core::naml_struct_set_field(res, response_fields::BODY, body as i64);

        res
    }
}

/// Get request method
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_method(req: *const NamlStruct) -> *mut NamlString {
    unsafe {
        naml_std_core::naml_struct_get_field(req, request_fields::METHOD) as *mut NamlString
    }
}

/// Set request method
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_method(
    req: *mut NamlStruct,
    method: *const NamlString,
) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::METHOD, method as i64);
    }
}

/// Get request path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_path(req: *const NamlStruct) -> *mut NamlString {
    unsafe {
        naml_std_core::naml_struct_get_field(req, request_fields::PATH) as *mut NamlString
    }
}

/// Set request path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_path(
    req: *mut NamlStruct,
    path: *const NamlString,
) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::PATH, path as i64);
    }
}

/// Get request headers (returns map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_headers(req: *const NamlStruct) -> i64 {
    unsafe { naml_std_core::naml_struct_get_field(req, request_fields::HEADERS) }
}

/// Set request headers (takes map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_headers(req: *mut NamlStruct, headers: i64) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::HEADERS, headers);
    }
}

/// Get request body
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_body(req: *const NamlStruct) -> *mut NamlArray {
    unsafe {
        naml_std_core::naml_struct_get_field(req, request_fields::BODY) as *mut NamlArray
    }
}

/// Set request body
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_body(
    req: *mut NamlStruct,
    body: *const NamlArray,
) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::BODY, body as i64);
    }
}

/// Get request URL params (returns map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_params(req: *const NamlStruct) -> i64 {
    unsafe { naml_std_core::naml_struct_get_field(req, request_fields::PARAMS) }
}

/// Set request URL params (takes map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_params(req: *mut NamlStruct, params: i64) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::PARAMS, params);
    }
}

/// Get request query parameters (returns map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_get_query(req: *const NamlStruct) -> i64 {
    unsafe { naml_std_core::naml_struct_get_field(req, request_fields::QUERY) }
}

/// Set request query parameters (takes map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_request_set_query(req: *mut NamlStruct, query: i64) {
    unsafe {
        naml_std_core::naml_struct_set_field(req, request_fields::QUERY, query);
    }
}

/// Get response status
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_get_status(res: *const NamlStruct) -> i64 {
    unsafe { naml_std_core::naml_struct_get_field(res, response_fields::STATUS) }
}

/// Set response status
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_set_status(res: *mut NamlStruct, status: i64) {
    unsafe {
        naml_std_core::naml_struct_set_field(res, response_fields::STATUS, status);
    }
}

/// Get response headers (returns map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_get_headers(res: *const NamlStruct) -> i64 {
    unsafe { naml_std_core::naml_struct_get_field(res, response_fields::HEADERS) }
}

/// Set response headers (takes map pointer as i64)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_set_headers(res: *mut NamlStruct, headers: i64) {
    unsafe {
        naml_std_core::naml_struct_set_field(res, response_fields::HEADERS, headers);
    }
}

/// Get response body
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_get_body(res: *const NamlStruct) -> *mut NamlArray {
    unsafe {
        naml_std_core::naml_struct_get_field(res, response_fields::BODY) as *mut NamlArray
    }
}

/// Set response body
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_response_set_body(
    res: *mut NamlStruct,
    body: *const NamlArray,
) {
    unsafe {
        naml_std_core::naml_struct_set_field(res, response_fields::BODY, body as i64);
    }
}

/// Helper to create NamlBytes from raw data
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

/// Helper to convert bytes array to Vec<u8>
pub(crate) unsafe fn array_to_vec(arr: *const NamlArray) -> Vec<u8> {
    if arr.is_null() {
        return Vec::new();
    }
    unsafe {
        let len = naml_std_core::naml_array_len(arr) as usize;
        let mut bytes = Vec::with_capacity(len);
        for i in 0..len {
            bytes.push(naml_std_core::naml_array_get(arr, i as i64) as u8);
        }
        bytes
    }
}

/// Helper to create array from bytes
pub(crate) unsafe fn vec_to_array(bytes: &[u8]) -> *mut NamlArray {
    unsafe {
        let arr = naml_std_core::naml_array_new(bytes.len());
        for &byte in bytes {
            naml_std_core::naml_array_push(arr, byte as i64);
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::string_from_naml;

    #[test]
    fn test_request_new() {
        unsafe {
            let req = naml_net_http_request_new();
            assert!(!req.is_null());

            // Check method is empty string
            let method = naml_net_http_request_get_method(req);
            assert!(!method.is_null());
            assert_eq!((*method).len, 0);

            // Check path is empty string
            let path = naml_net_http_request_get_path(req);
            assert!(!path.is_null());
            assert_eq!((*path).len, 0);

            // Check headers is null
            let headers = naml_net_http_request_get_headers(req);
            assert_eq!(headers, 0);

            // Check body is empty array
            let body = naml_net_http_request_get_body(req);
            assert!(!body.is_null());
            assert_eq!(naml_std_core::naml_array_len(body), 0);
        }
    }

    #[test]
    fn test_request_set_method() {
        unsafe {
            let req = naml_net_http_request_new();

            let method = naml_string_new(b"POST".as_ptr(), 4);
            naml_net_http_request_set_method(req, method);

            let got_method = naml_net_http_request_get_method(req);
            let method_str = string_from_naml(got_method);
            assert_eq!(method_str, "POST");
        }
    }

    #[test]
    fn test_request_set_path() {
        unsafe {
            let req = naml_net_http_request_new();

            let path = naml_string_new(b"/api/users".as_ptr(), 10);
            naml_net_http_request_set_path(req, path);

            let got_path = naml_net_http_request_get_path(req);
            let path_str = string_from_naml(got_path);
            assert_eq!(path_str, "/api/users");
        }
    }

    #[test]
    fn test_response_new() {
        unsafe {
            let res = naml_net_http_response_new();
            assert!(!res.is_null());

            // Check status defaults to 200
            let status = naml_net_http_response_get_status(res);
            assert_eq!(status, 200);

            // Check headers is null
            let headers = naml_net_http_response_get_headers(res);
            assert_eq!(headers, 0);

            // Check body is empty array
            let body = naml_net_http_response_get_body(res);
            assert!(!body.is_null());
            assert_eq!(naml_std_core::naml_array_len(body), 0);
        }
    }

    #[test]
    fn test_response_set_status() {
        unsafe {
            let res = naml_net_http_response_new();

            naml_net_http_response_set_status(res, 404);

            let status = naml_net_http_response_get_status(res);
            assert_eq!(status, 404);
        }
    }

    #[test]
    fn test_response_create() {
        unsafe {
            let body = naml_std_core::naml_array_new(5);
            for i in 0..5 {
                naml_std_core::naml_array_push(body, i);
            }

            let res = naml_net_http_response_create(201, 0, body);
            assert!(!res.is_null());

            assert_eq!(naml_net_http_response_get_status(res), 201);
            assert_eq!(naml_net_http_response_get_headers(res), 0);
            assert_eq!(naml_net_http_response_get_body(res), body);
        }
    }

    #[test]
    fn test_array_to_vec() {
        unsafe {
            let arr = naml_std_core::naml_array_new(4);
            naml_std_core::naml_array_push(arr, b'T' as i64);
            naml_std_core::naml_array_push(arr, b'e' as i64);
            naml_std_core::naml_array_push(arr, b's' as i64);
            naml_std_core::naml_array_push(arr, b't' as i64);

            let vec = array_to_vec(arr);
            assert_eq!(vec, b"Test");
        }
    }

    #[test]
    fn test_vec_to_array() {
        unsafe {
            let bytes = b"Hello";
            let arr = vec_to_array(bytes);

            assert_eq!(naml_std_core::naml_array_len(arr), 5);
            for (i, &byte) in bytes.iter().enumerate() {
                assert_eq!(naml_std_core::naml_array_get(arr, i as i64), byte as i64);
            }
        }
    }
}
