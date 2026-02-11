//!
//! HTTP Client Implementation
//!
//! Provides HTTP client functions for naml programs.
//!
//! ## Functions
//!
//! - `naml_net_http_client_get` - HTTP GET request with optional headers
//! - `naml_net_http_client_post` - HTTP POST request with optional headers
//! - `naml_net_http_client_put` - HTTP PUT request with optional headers
//! - `naml_net_http_client_patch` - HTTP PATCH request with optional headers
//! - `naml_net_http_client_delete` - HTTP DELETE request with optional headers
//! - `naml_net_http_client_set_timeout` - Set default timeout
//!
//! All HTTP methods accept an optional headers parameter (`option<map<string, string>>`).
//! Pass `none` to use default headers, or `some(headers_map)` to set custom headers.
//!
//! ## Note
//!
//! Supports both HTTP and HTTPS URLs transparently via rustls.
//!

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper::body::Bytes;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tokio::runtime::Runtime;

use naml_std_core::{NamlBytes, NamlMap, NamlString, NamlStruct};

use super::types::{
    naml_net_http_response_new, naml_net_http_response_set_body, naml_net_http_response_set_status,
    vec_to_array,
};

/// Helper to convert NamlBytes to Vec<u8>
unsafe fn bytes_to_vec(bytes: *const NamlBytes) -> Vec<u8> {
    if bytes.is_null() {
        return Vec::new();
    }
    unsafe {
        let len = (*bytes).len;
        let data = (*bytes).data.as_ptr();
        std::slice::from_raw_parts(data, len).to_vec()
    }
}

/// Represents an option struct layout:
/// - offset 0: tag (i32) - 0 = none, 1 = some
/// - offset 8: value (i64) - the actual value pointer when some
#[repr(C)]
struct NamlOption {
    tag: i32,
    _padding: i32,
    value: i64,
}

/// Helper to extract headers from an option<map<string, string>>
/// Returns a Vec of (header_name, header_value) pairs
unsafe fn extract_headers(headers_opt: *const NamlOption) -> Vec<(String, String)> {
    if headers_opt.is_null() {
        return Vec::new();
    }

    unsafe {
        // Check if it's none (tag == 0)
        let tag = (*headers_opt).tag;
        if tag == 0 {
            return Vec::new();
        }

        // It's some, extract the map pointer from the value field
        let map = (*headers_opt).value as *const NamlMap;
        if map.is_null() {
            return Vec::new();
        }

        let mut result = Vec::new();

        // Iterate over map entries
        let capacity = (*map).capacity;
        let entries = (*map).entries;

        // Sanity check to prevent huge allocations
        if capacity > 10000 {
            return Vec::new();
        }

        for i in 0..capacity {
            let entry = entries.add(i);

            if (*entry).occupied {
                // Key and value are pointers to NamlString
                let key_ptr = (*entry).key as *const NamlString;
                let val_ptr = (*entry).value as *const NamlString;

                if !key_ptr.is_null() && !val_ptr.is_null() {
                    // Validate string lengths to detect corrupted pointers
                    let key_len = (*key_ptr).len;
                    let val_len = (*val_ptr).len;

                    if key_len > 10000 || val_len > 10000 {
                        // Skip entries with invalid string lengths
                        continue;
                    }

                    let key_str = (*key_ptr).as_str().to_string();
                    let val_str = (*val_ptr).as_str().to_string();
                    result.push((key_str, val_str));
                }
            }
        }

        result
    }
}

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
    custom_headers: Vec<(String, String)>,
) -> *mut NamlStruct {
    let timeout_ms = DEFAULT_TIMEOUT_MS.load(Ordering::SeqCst);
    let timeout = Duration::from_millis(timeout_ms);

    let runtime = get_runtime();

    let method_clone = method.to_string();
    let url_clone = url.to_string();

    let result: Result<(i64, Vec<u8>), std::io::Error> = runtime.block_on(async move {
        // Parse URL
        let uri: hyper::Uri = url_clone
            .parse()
            .map_err(|e: hyper::http::uri::InvalidUri| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid URL: {}", e),
                )
            })?;

        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls_config = rustls::ClientConfig::builder_with_provider(
                rustls::crypto::ring::default_provider().into(),
            )
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build(connector);

        // Build request with default headers
        let body_bytes = body.unwrap_or_default();
        let mut req_builder = Request::builder()
            .method(method_clone.as_str())
            .uri(uri)
            .header("User-Agent", "naml-http-client/0.1")
            .header("Accept", "*/*");

        // Add custom headers (they can override defaults)
        for (name, value) in custom_headers {
            req_builder = req_builder.header(name, value);
        }

        let req = req_builder
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
            .map_err(|e: hyper_util::client::legacy::Error| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

        // Extract status
        let status = response.status().as_u16() as i64;

        // Read body
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e: hyper::Error| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?
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

/// HTTP GET request with optional headers
/// headers_opt is a pointer to option<map<string, string>> (opaque)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_get(
    url: *const NamlString,
    headers_opt: *const u8,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let headers = unsafe { extract_headers(headers_opt as *const NamlOption) };
    do_request("GET", &url_str, None, headers)
}

/// HTTP POST request with optional headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_post(
    url: *const NamlString,
    body: *const NamlBytes,
    headers_opt: *const u8,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { bytes_to_vec(body) };
    let headers = unsafe { extract_headers(headers_opt as *const NamlOption) };
    do_request("POST", &url_str, Some(body_bytes), headers)
}

/// HTTP PUT request with optional headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_put(
    url: *const NamlString,
    body: *const NamlBytes,
    headers_opt: *const u8,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { bytes_to_vec(body) };
    let headers = unsafe { extract_headers(headers_opt as *const NamlOption) };
    do_request("PUT", &url_str, Some(body_bytes), headers)
}

/// HTTP PATCH request with optional headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_patch(
    url: *const NamlString,
    body: *const NamlBytes,
    headers_opt: *const u8,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let body_bytes = unsafe { bytes_to_vec(body) };
    let headers = unsafe { extract_headers(headers_opt as *const NamlOption) };
    do_request("PATCH", &url_str, Some(body_bytes), headers)
}

/// HTTP DELETE request with optional headers
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_delete(
    url: *const NamlString,
    headers_opt: *const u8,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let headers = unsafe { extract_headers(headers_opt as *const NamlOption) };
    do_request("DELETE", &url_str, None, headers)
}

/// HTTP GET request with a custom CA certificate for TLS verification
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_client_get_tls(
    url: *const NamlString,
    ca_path: *const NamlString,
) -> *mut NamlStruct {
    let url_str = unsafe { string_from_naml(url) };
    let ca_str = unsafe { string_from_naml(ca_path) };

    let timeout_ms = DEFAULT_TIMEOUT_MS.load(Ordering::SeqCst);
    let timeout = Duration::from_millis(timeout_ms);

    let runtime = get_runtime();

    let result: Result<(i64, Vec<u8>), String> = runtime.block_on(async move {
        let ca_file = std::fs::File::open(&ca_str)
            .map_err(|e| format!("failed to open CA file '{}': {}", ca_str, e))?;
        let mut ca_reader = std::io::BufReader::new(ca_file);
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let custom_certs: Vec<_> = rustls_pemfile::certs(&mut ca_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("failed to parse CA certificate: {}", e))?;
        for cert in custom_certs {
            root_store
                .add(cert)
                .map_err(|e| format!("failed to add CA certificate: {}", e))?;
        }

        let tls_config = rustls::ClientConfig::builder_with_provider(
                rustls::crypto::ring::default_provider().into(),
            )
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_or_http()
            .enable_http1()
            .build();
        let client: Client<_, Full<Bytes>> =
            Client::builder(TokioExecutor::new()).build(connector);

        let uri: hyper::Uri = url_str
            .parse()
            .map_err(|e: hyper::http::uri::InvalidUri| format!("Invalid URL: {}", e))?;

        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .header("User-Agent", "naml-http-client/0.1")
            .header("Accept", "*/*")
            .body(Full::new(Bytes::new()))
            .map_err(|e| e.to_string())?;

        let response = tokio::time::timeout(timeout, client.request(req))
            .await
            .map_err(|_| format!("Request timed out after {}ms", timeout_ms))?
            .map_err(|e| e.to_string())?;

        let status = response.status().as_u16() as i64;
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| e.to_string())?
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
        Err(msg) => {
            crate::errors::throw_tls_error(&msg);
            std::ptr::null_mut()
        }
    }
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
            // Create a none option (tag = 0)
            let none_opt = NamlOption {
                tag: 0,
                _padding: 0,
                value: 0,
            };
            let result = naml_net_http_client_get(url, &none_opt as *const NamlOption as *const u8);
            assert!(result.is_null(), "Should fail with invalid URL");
        }
    }
}
