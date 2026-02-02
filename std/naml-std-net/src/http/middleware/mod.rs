//!
//! HTTP Middleware Module
//!
//! Provides built-in middleware for HTTP servers using tower-http.
//! Middleware are stored as configuration and applied as tower-http layers
//! when the server starts.
//!
//! ## Available Middleware
//!
//! - `logger` - Request/response logging (tower-http TraceLayer)
//! - `timeout` - Request timeout handling (tower-http TimeoutLayer)
//! - `recover` - Panic recovery (tower-http CatchPanicLayer)
//! - `cors` - Cross-Origin Resource Sharing (tower-http CorsLayer)
//! - `rate_limit` - Rate limiting (tower RateLimitLayer)
//! - `compress` - Response compression (tower-http CompressionLayer)
//! - `request_id` - Request ID generation (tower-http SetRequestIdLayer)
//!

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::RwLock;

use naml_std_core::NamlArray;

use crate::errors::string_from_naml;

/// Middleware configuration types
#[derive(Clone)]
pub enum MiddlewareConfig {
    Logger,
    Timeout { ms: u64 },
    Recover,
    Cors { origins: Vec<String> },
    RateLimit { rps: u64 },
    Compress,
    RequestId,
}

/// Global middleware registry
static NEXT_MW_HANDLE: AtomicI64 = AtomicI64::new(1);
static MIDDLEWARE_CONFIGS: std::sync::OnceLock<RwLock<std::collections::HashMap<i64, MiddlewareConfig>>> =
    std::sync::OnceLock::new();

fn get_middleware_configs() -> &'static RwLock<std::collections::HashMap<i64, MiddlewareConfig>> {
    MIDDLEWARE_CONFIGS.get_or_init(|| RwLock::new(std::collections::HashMap::new()))
}

fn next_mw_handle() -> i64 {
    NEXT_MW_HANDLE.fetch_add(1, Ordering::SeqCst)
}

/// Get middleware config by handle
pub fn get_middleware_config(handle: i64) -> Option<MiddlewareConfig> {
    let configs = get_middleware_configs().read().unwrap();
    configs.get(&handle).cloned()
}

/// Create a logger middleware (tower-http TraceLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_logger() -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::Logger);
    handle
}

/// Create a timeout middleware (tower-http TimeoutLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_timeout(ms: i64) -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::Timeout { ms: ms.max(0) as u64 });
    handle
}

/// Create a recover middleware (tower-http CatchPanicLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_recover() -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::Recover);
    handle
}

/// Create a CORS middleware (tower-http CorsLayer)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_middleware_cors(origins: *const NamlArray) -> i64 {
    let origins_vec = unsafe { array_to_string_vec(origins) };
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::Cors { origins: origins_vec });
    handle
}

/// Create a rate limit middleware (tower RateLimitLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_rate_limit(requests_per_second: i64) -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::RateLimit { rps: requests_per_second.max(1) as u64 });
    handle
}

/// Create a compress middleware (tower-http CompressionLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_compress() -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::Compress);
    handle
}

/// Create a request ID middleware (tower-http SetRequestIdLayer)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_middleware_request_id() -> i64 {
    let handle = next_mw_handle();
    let mut configs = get_middleware_configs().write().unwrap();
    configs.insert(handle, MiddlewareConfig::RequestId);
    handle
}

/// Convert NamlArray of strings to Vec<String>
unsafe fn array_to_string_vec(arr: *const NamlArray) -> Vec<String> {
    if arr.is_null() {
        return Vec::new();
    }

    unsafe {
        let arr_ref = &*arr;
        let len = arr_ref.len as usize;
        let data = arr_ref.data as *const *const naml_std_core::NamlString;

        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let str_ptr = *data.add(i);
            if !str_ptr.is_null() {
                result.push(string_from_naml(str_ptr));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let handle = naml_net_http_middleware_logger();
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::Logger)));
    }

    #[test]
    fn test_timeout_creation() {
        let handle = naml_net_http_middleware_timeout(5000);
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::Timeout { ms: 5000 })));
    }

    #[test]
    fn test_recover_creation() {
        let handle = naml_net_http_middleware_recover();
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::Recover)));
    }

    #[test]
    fn test_rate_limit_creation() {
        let handle = naml_net_http_middleware_rate_limit(100);
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::RateLimit { rps: 100 })));
    }

    #[test]
    fn test_compress_creation() {
        let handle = naml_net_http_middleware_compress();
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::Compress)));
    }

    #[test]
    fn test_request_id_creation() {
        let handle = naml_net_http_middleware_request_id();
        assert!(handle > 0);
        let config = get_middleware_config(handle);
        assert!(matches!(config, Some(MiddlewareConfig::RequestId)));
    }
}
