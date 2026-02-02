//!
//! HTTP Client Implementation
//!
//! Provides HTTP client functions for naml programs.
//!
//! ## Functions
//!
//! - `naml_net_http_client_get` - HTTP GET request
//! - `naml_net_http_client_post` - HTTP POST request
//! - `naml_net_http_client_put` - HTTP PUT request
//! - `naml_net_http_client_patch` - HTTP PATCH request
//! - `naml_net_http_client_delete` - HTTP DELETE request
//! - `naml_net_http_client_get_with_headers` - GET with custom headers
//! - `naml_net_http_client_post_with_headers` - POST with custom headers
//! - `naml_net_http_client_set_timeout` - Set default timeout
//!
//! ## Note
//!
//! Currently supports HTTP only. HTTPS support requires additional TLS dependencies.
//!

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::Request;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::runtime::Runtime;

use naml_std_core::{NamlArray, NamlString, NamlStruct};

use super::types::{
    array_to_vec, naml_net_http_response_new, naml_net_http_response_set_body,
    naml_net_http_response_set_status, vec_to_array,
};
use crate::errors::{string_from_naml, throw_network_error, throw_timeout_error};

/// Default timeout in milliseconds (30 seconds)
static DEFAULT_TIMEOUT_MS: AtomicU64 = AtomicU64::new(30000);

/// Get or create the tokio runtime for HTTP operations
fn get_runtime() -> &'static Runtime {
    use std::sync::OnceLock;
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Set the default timeout for HTTP requests
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_client_set_timeout(ms: i64) {
    let ms = ms.max(0) as u64;
    DEFAULT_TIMEOUT_MS.store(ms, Ordering::SeqCst);
}

/// Perform an HTTP request and return a response struct
fn do_request(
    method: &str,
    url: &str,
    body: Option<Vec<u8>>,
) -> *mut NamlStruct {
    let timeout_ms = DEFAULT_TIMEOUT_MS.load(Ordering::SeqCst);
    let timeout = Duration::from_millis(timeout_ms);

    let runtime = get_runtime();

    let method_clone = method.to_string();
    let url_clone = url.to_string();

    let result: Result<(i64, Vec<u8>), std::io::Error> = runtime.block_on(async move {
        // Parse URL
        let uri: hyper::Uri = url_clone.parse().map_err(|e: hyper::http::uri::InvalidUri| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid URL: {}", e))
        })?;

        // Create HTTP connector (no TLS)
        let client: Client<_, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build_http();

        // Build request
        let body_bytes = body.unwrap_or_default();
        let req = Request::builder()
            .method(method_clone.as_str())
            .uri(uri)
            .header("User-Agent", "naml-http-client/0.1")
            .header("Accept", "*/*")
            .body(Full::new(Bytes::from(body_bytes)))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        // Send request with timeout
        let response = tokio::time::timeout(timeout, client.request(req))
            .await
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Request timed out after {}ms", timeout_ms),
                )
            })?
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // Extract status
        let status = response.status().as_u16() as i64;

        // Read body
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
            .to_bytes()
            .to_vec();

        Ok((status, body_bytes))
    });

    match result {
        Ok((status, body_bytes)) => unsafe {
            let response = naml_net_http_response_new();
            naml_net_http_response_set_status(response, status);
            let body_arr = vec_to_array(&body_bytes);
            naml_net_http_response_set_body(response, body_arr);
            response
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::TimedOut {
                throw_timeout_error(&e.to_string(), timeout_ms as i64);
            } else {
                throw_network_error(e);
            }
            std::ptr::null_mut()
        }
    }
}

/// HTTP GET request
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_get(url: *const NamlString) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    do_request("GET", &url_str, None)
}

/// HTTP POST request
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_post(
    url: *const NamlString,
    body: *const NamlArray,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { array_to_vec(body) };
    do_request("POST", &url_str, Some(body_bytes))
}

/// HTTP PUT request
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_put(
    url: *const NamlString,
    body: *const NamlArray,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { array_to_vec(body) };
    do_request("PUT", &url_str, Some(body_bytes))
}

/// HTTP PATCH request
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_patch(
    url: *const NamlString,
    body: *const NamlArray,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { array_to_vec(body) };
    do_request("PATCH", &url_str, Some(body_bytes))
}

/// HTTP DELETE request
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_delete(url: *const NamlString) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    do_request("DELETE", &url_str, None)
}

/// HTTP GET request with custom headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_get_with_headers(
    url: *const NamlString,
    _headers: i64,
) -> *mut NamlStruct {
    // TODO: Parse headers map and add to request
    let url_str = unsafe { string_from_naml(url) };
    do_request("GET", &url_str, None)
}

/// HTTP POST request with custom headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_post_with_headers(
    url: *const NamlString,
    body: *const NamlArray,
    _headers: i64,
) -> *mut NamlStruct {
    // TODO: Parse headers map and add to request
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { array_to_vec(body) };
    do_request("POST", &url_str, Some(body_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use naml_std_core::naml_string_new;

    #[test]
    fn test_set_timeout() {
        naml_net_http_client_set_timeout(5000);
        assert_eq!(DEFAULT_TIMEOUT_MS.load(Ordering::SeqCst), 5000);

        // Reset to default
        naml_net_http_client_set_timeout(30000);
    }

    #[test]
    fn test_invalid_url() {
        unsafe {
            let url = naml_string_new(b"not-a-valid-url".as_ptr(), 15);
            let result = naml_net_http_client_get(url);
            assert!(result.is_null(), "Should fail with invalid URL");
        }
    }
}
