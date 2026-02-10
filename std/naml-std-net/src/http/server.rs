//!
//! HTTP Server Implementation
//!
//! Provides chi-style HTTP server routing for naml programs.
//!
//! ## Functions
//!
//! - `naml_net_http_server_open_router` - Create a new router
//! - `naml_net_http_server_get` - Register GET handler
//! - `naml_net_http_server_post` - Register POST handler
//! - `naml_net_http_server_put` - Register PUT handler
//! - `naml_net_http_server_patch` - Register PATCH handler
//! - `naml_net_http_server_delete` - Register DELETE handler
//! - `naml_net_http_server_with` - Add middleware to router
//! - `naml_net_http_server_group` - Create route group
//! - `naml_net_http_server_mount` - Mount sub-router
//! - `naml_net_http_server_serve` - Start HTTP server
//!
//! ## Note
//!
//! Handlers are naml function pointers: fn(request) -> response
//! Middleware are naml function pointers: fn(handler) -> handler
//!

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

use naml_std_core::{HeapTag, NamlArray, NamlBytes, NamlString, NamlStruct};

use super::types::{
    array_to_vec, create_bytes_from, naml_net_http_response_create, naml_net_http_response_get_body,
    naml_net_http_response_get_status, vec_to_array,
};
use crate::errors::{string_from_naml, throw_network_error};

/// Handler function type (naml function pointer)
type HandlerFn = extern "C" fn(*mut NamlStruct) -> *mut NamlStruct;

/// Route definition
#[derive(Clone)]
struct Route {
    pattern: String,
    method: String,
    handler: HandlerFn,
    param_names: Vec<String>,
}

/// Router structure
struct Router {
    routes: Vec<Route>,
    middleware_handles: Vec<i64>,
    prefix: String,
}

impl Router {
    fn new() -> Self {
        Self {
            routes: Vec::new(),
            middleware_handles: Vec::new(),
            prefix: String::new(),
        }
    }

    fn with_prefix(prefix: String) -> Self {
        Self {
            routes: Vec::new(),
            middleware_handles: Vec::new(),
            prefix,
        }
    }

    fn add_route(&mut self, method: &str, pattern: &str, handler: HandlerFn) {
        let full_pattern = if self.prefix.is_empty() {
            pattern.to_string()
        } else {
            format!("{}{}", self.prefix, pattern)
        };

        let param_names = extract_param_names(&full_pattern);

        self.routes.push(Route {
            pattern: full_pattern,
            method: method.to_string(),
            handler,
            param_names,
        });
    }

    fn add_middleware(&mut self, mw_handle: i64) {
        self.middleware_handles.push(mw_handle);
    }
}

/// Frozen (immutable) router snapshot for zero-lock request handling.
/// Created once at serve-time; shared across all worker tasks via Arc.
struct FrozenRouter {
    exact_routes: Vec<(String, String, HandlerFn)>,
    param_routes: Vec<Route>,
    has_logger: bool,
    timeout_ms: Option<u64>,
    has_recover: bool,
    has_compress: bool,
}

impl FrozenRouter {
    fn from_router(router: &Router) -> Self {
        use super::middleware::{get_middleware_config, MiddlewareConfig};

        let mut exact_routes = Vec::new();
        let mut param_routes = Vec::new();
        for route in &router.routes {
            if route.param_names.is_empty() {
                exact_routes.push((
                    route.method.clone(),
                    route.pattern.clone(),
                    route.handler,
                ));
            } else {
                param_routes.push(route.clone());
            }
        }

        let mut has_logger = false;
        let mut timeout_ms = None;
        let mut has_recover = false;
        let mut has_compress = false;

        for handle in &router.middleware_handles {
            if let Some(config) = get_middleware_config(*handle) {
                match config {
                    MiddlewareConfig::Logger => has_logger = true,
                    MiddlewareConfig::Timeout { ms } => timeout_ms = Some(ms),
                    MiddlewareConfig::Recover => has_recover = true,
                    MiddlewareConfig::Compress => has_compress = true,
                    _ => {}
                }
            }
        }

        FrozenRouter {
            exact_routes,
            param_routes,
            has_logger,
            timeout_ms,
            has_recover,
            has_compress,
        }
    }
}

/// Extract parameter names from a pattern like "/users/{id}/posts/{post_id}"
fn extract_param_names(pattern: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_param = false;
    let mut param_name = String::new();

    for c in pattern.chars() {
        if c == '{' {
            in_param = true;
            param_name.clear();
        } else if c == '}' {
            if in_param && !param_name.is_empty() {
                names.push(param_name.clone());
            }
            in_param = false;
        } else if in_param {
            param_name.push(c);
        }
    }

    names
}

/// Convert pattern to regex-like matcher and extract param values
fn match_route(pattern: &str, path: &str, param_names: &[String]) -> Option<HashMap<String, String>> {
    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_parts.len() != path_parts.len() {
        return None;
    }

    let mut params = HashMap::new();
    let mut param_idx = 0;

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            if param_idx < param_names.len() {
                params.insert(param_names[param_idx].clone(), path_part.to_string());
                param_idx += 1;
            }
        } else if *pattern_part != *path_part {
            return None;
        }
    }

    Some(params)
}

/// Global router registry
static NEXT_ROUTER_HANDLE: AtomicI64 = AtomicI64::new(1);
static ROUTERS: std::sync::OnceLock<RwLock<HashMap<i64, Arc<Mutex<Router>>>>> =
    std::sync::OnceLock::new();

fn get_routers() -> &'static RwLock<HashMap<i64, Arc<Mutex<Router>>>> {
    ROUTERS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn next_router_handle() -> i64 {
    NEXT_ROUTER_HANDLE.fetch_add(1, Ordering::SeqCst)
}

/// Get or create the tokio runtime for HTTP server
fn get_runtime() -> &'static Runtime {
    use std::sync::OnceLock;
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

/// Create a new router
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_server_open_router() -> i64 {
    let handle = next_router_handle();
    let router = Arc::new(Mutex::new(Router::new()));

    let mut routers = get_routers().write().unwrap();
    routers.insert(handle, router);

    handle
}

/// Register a GET handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_get(
    router_handle: i64,
    pattern: *const NamlString,
    handler: HandlerFn,
) {
    let pattern_str = unsafe { string_from_naml(pattern) };
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_route("GET", &pattern_str, handler);
    }
}

/// Register a POST handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_post(
    router_handle: i64,
    pattern: *const NamlString,
    handler: HandlerFn,
) {
    let pattern_str = unsafe { string_from_naml(pattern) };
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_route("POST", &pattern_str, handler);
    }
}

/// Register a PUT handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_put(
    router_handle: i64,
    pattern: *const NamlString,
    handler: HandlerFn,
) {
    let pattern_str = unsafe { string_from_naml(pattern) };
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_route("PUT", &pattern_str, handler);
    }
}

/// Register a PATCH handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_patch(
    router_handle: i64,
    pattern: *const NamlString,
    handler: HandlerFn,
) {
    let pattern_str = unsafe { string_from_naml(pattern) };
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_route("PATCH", &pattern_str, handler);
    }
}

/// Register a DELETE handler
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_delete(
    router_handle: i64,
    pattern: *const NamlString,
    handler: HandlerFn,
) {
    let pattern_str = unsafe { string_from_naml(pattern) };
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_route("DELETE", &pattern_str, handler);
    }
}

/// Add middleware to router (middleware_handle is from middleware::* functions)
#[unsafe(no_mangle)]
pub extern "C" fn naml_net_http_server_with(router_handle: i64, middleware_handle: i64) {
    let routers = get_routers().read().unwrap();
    if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().add_middleware(middleware_handle);
    }
}

/// Create a route group with prefix
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_group(
    router_handle: i64,
    prefix: *const NamlString,
) -> i64 {
    let prefix_str = unsafe { string_from_naml(prefix) };

    let routers = get_routers().read().unwrap();
    let parent_prefix = if let Some(router) = routers.get(&router_handle) {
        router.lock().unwrap().prefix.clone()
    } else {
        String::new()
    };
    drop(routers);

    let full_prefix = format!("{}{}", parent_prefix, prefix_str);
    let group_handle = next_router_handle();
    let group_router = Arc::new(Mutex::new(Router::with_prefix(full_prefix)));

    let mut routers = get_routers().write().unwrap();
    routers.insert(group_handle, group_router);

    group_handle
}

/// Mount a sub-router at a prefix
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_mount(
    router_handle: i64,
    prefix: *const NamlString,
    sub_router_handle: i64,
) {
    let prefix_str = unsafe { string_from_naml(prefix) };

    let routers = get_routers().read().unwrap();
    let sub_routes = if let Some(sub_router) = routers.get(&sub_router_handle) {
        let sub = sub_router.lock().unwrap();
        sub.routes.clone()
    } else {
        return;
    };

    if let Some(router) = routers.get(&router_handle) {
        let mut r = router.lock().unwrap();
        for route in sub_routes {
            let new_pattern = format!("{}{}", prefix_str, route.pattern);
            let param_names = extract_param_names(&new_pattern);
            r.routes.push(Route {
                pattern: new_pattern,
                method: route.method,
                handler: route.handler,
                param_names,
            });
        }
    }
}

/// Start HTTP server
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_serve(
    address: *const NamlString,
    router_handle: i64,
) {
    let addr_str = unsafe { string_from_naml(address) };
    let runtime = get_runtime();

    let frozen = {
        let routers = get_routers().read().unwrap();
        let router_arc = match routers.get(&router_handle) {
            Some(r) => Arc::clone(r),
            None => {
                drop(routers);
                throw_network_error(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Router not found",
                ));
                return;
            }
        };
        drop(routers);
        let router_guard = router_arc.lock().unwrap();
        Arc::new(FrozenRouter::from_router(&router_guard))
    };

    let result = runtime.block_on(async move {
        let addr: SocketAddr = if addr_str.starts_with(':') {
            format!("0.0.0.0{}", addr_str).parse()
        } else {
            addr_str.parse()
        }
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        let listener = TcpListener::bind(addr).await?;

        loop {
            let (stream, _) = listener.accept().await?;
            let _ = stream.set_nodelay(true);
            let io = TokioIo::new(stream);
            let frozen_clone = Arc::clone(&frozen);

            tokio::spawn(async move {
                let service = service_fn(move |req: Request<Incoming>| {
                    let frozen = Arc::clone(&frozen_clone);
                    async move { handle_request(req, &frozen).await }
                });

                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    eprintln!("Server error: {}", e);
                }
            });
        }

        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    });

    if let Err(e) = result {
        throw_network_error(e);
    }
}

/// Handle incoming HTTP request
async fn handle_request(
    req: Request<Incoming>,
    frozen: &FrozenRouter,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let start = if frozen.has_logger || frozen.timeout_ms.is_some() {
        Some(std::time::Instant::now())
    } else {
        None
    };

    let (parts, body) = req.into_parts();
    let skip_body = parts.method == Method::GET || parts.method == Method::HEAD;
    let method = parts.method.as_str();
    let path = parts.uri.path();
    let query_string = parts.uri.query().unwrap_or("");

    if let (Some(ms), Some(start)) = (frozen.timeout_ms, &start) {
        if start.elapsed().as_millis() > ms as u128 {
            if frozen.has_logger {
                eprintln!("[HTTP] {} {} -> 408 (timeout)", method, path);
            }
            return Ok(Response::builder()
                .status(408)
                .header("content-length", 15)
                .body(Full::new(Bytes::from_static(b"Request Timeout")))
                .unwrap());
        }
    }

    let mut matched_handler: Option<HandlerFn> = None;
    let mut params: HashMap<String, String> = HashMap::new();

    for (route_method, route_path, handler) in &frozen.exact_routes {
        if route_method == method && route_path == path {
            matched_handler = Some(*handler);
            break;
        }
    }

    if matched_handler.is_none() {
        for route in &frozen.param_routes {
            if route.method == method {
                if let Some(p) = match_route(&route.pattern, path, &route.param_names) {
                    matched_handler = Some(route.handler);
                    params = p;
                    break;
                }
            }
        }
    }

    let (status, mut response_body) = if let Some(handler) = matched_handler {
        let body_bytes = if skip_body {
            drop(body);
            Vec::new()
        } else {
            match body.collect().await {
                Ok(collected) => collected.to_bytes().to_vec(),
                Err(_) => Vec::new(),
            }
        };

        let naml_request =
            unsafe { create_naml_request(method, path, &body_bytes, &params, query_string) };

        let result = if frozen.has_recover {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(naml_request)))
        } else {
            Ok(handler(naml_request))
        };

        match result {
            Ok(naml_response) if !naml_response.is_null() => unsafe {
                let status = naml_net_http_response_get_status(naml_response);
                let body_ptr = naml_net_http_response_get_body(naml_response);
                let body_vec = if body_ptr.is_null() {
                    Vec::new()
                } else if (*(body_ptr as *const NamlBytes)).header.tag == HeapTag::Bytes {
                    let b = body_ptr as *const NamlBytes;
                    std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len).to_vec()
                } else {
                    array_to_vec(body_ptr)
                };
                (status as u16, body_vec)
            },
            Ok(_) => (500, b"Internal Server Error".to_vec()),
            Err(_) => {
                eprintln!("[HTTP] Recovered from panic in request handler");
                (500, b"Internal Server Error".to_vec())
            }
        }
    } else {
        (404, b"Not Found".to_vec())
    };

    if frozen.has_compress && response_body.len() >= 1024 {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        if encoder.write_all(&response_body).is_ok() {
            if let Ok(compressed) = encoder.finish() {
                if compressed.len() < response_body.len() {
                    response_body = compressed;
                }
            }
        }
    }

    if frozen.has_logger {
        if let Some(start) = start {
            let elapsed = start.elapsed();
            eprintln!("[HTTP] {} {} -> {} ({:.2?})", method, path, status, elapsed);
        }
    }

    Ok(Response::builder()
        .status(status)
        .header("content-length", response_body.len())
        .body(Full::new(Bytes::from(response_body)))
        .unwrap())
}

/// Create a naml request struct from HTTP request data.
/// Builds struct directly — avoids 3 dummy allocations from naml_net_http_request_new.
unsafe fn create_naml_request(
    method: &str,
    path: &str,
    body: &[u8],
    _params: &HashMap<String, String>,
    _query_string: &str,
) -> *mut NamlStruct {
    unsafe {
        let request = naml_std_core::naml_struct_new(
            super::types::TYPE_ID_REQUEST,
            super::types::request_fields::FIELD_COUNT,
        );

        let method_ptr = naml_std_core::naml_string_new(method.as_ptr(), method.len());
        naml_std_core::naml_struct_set_field(
            request,
            super::types::request_fields::METHOD,
            method_ptr as i64,
        );

        let path_ptr = naml_std_core::naml_string_new(path.as_ptr(), path.len());
        naml_std_core::naml_struct_set_field(
            request,
            super::types::request_fields::PATH,
            path_ptr as i64,
        );

        naml_std_core::naml_struct_set_field(request, super::types::request_fields::HEADERS, 0);
        naml_std_core::naml_struct_set_field(request, super::types::request_fields::PARAMS, 0);
        naml_std_core::naml_struct_set_field(request, super::types::request_fields::QUERY, 0);

        if body.is_empty() {
            naml_std_core::naml_struct_set_field(request, super::types::request_fields::BODY, 0);
        } else {
            let body_arr = vec_to_array(body);
            naml_std_core::naml_struct_set_field(
                request,
                super::types::request_fields::BODY,
                body_arr as i64,
            );
        }

        request
    }
}

/// Create a text/JSON response from a status code and string body.
/// Reads NamlString data directly and copies into NamlBytes (1 alloc + 1 memcpy).
/// Must copy because the handler may decref the source string after returning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_http_server_text_response(
    status: i64,
    body: *const NamlString,
) -> *mut NamlStruct {
    unsafe {
        if body.is_null() {
            return naml_net_http_response_create(status, 0, std::ptr::null_mut());
        }
        let body_bytes = create_bytes_from((*body).data.as_ptr(), (*body).len);
        naml_net_http_response_create(status, 0, body_bytes as *mut NamlArray)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_param_names() {
        let names = extract_param_names("/users/{id}");
        assert_eq!(names, vec!["id"]);

        let names = extract_param_names("/users/{user_id}/posts/{post_id}");
        assert_eq!(names, vec!["user_id", "post_id"]);

        let names = extract_param_names("/static/path");
        assert!(names.is_empty());
    }

    #[test]
    fn test_match_route() {
        let pattern = "/users/{id}";
        let param_names = vec!["id".to_string()];

        let result = match_route(pattern, "/users/123", &param_names);
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        let result = match_route(pattern, "/users/123/extra", &param_names);
        assert!(result.is_none());

        let result = match_route(pattern, "/posts/123", &param_names);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_route_multiple_params() {
        let pattern = "/users/{user_id}/posts/{post_id}";
        let param_names = vec!["user_id".to_string(), "post_id".to_string()];

        let result = match_route(pattern, "/users/42/posts/99", &param_names);
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("user_id"), Some(&"42".to_string()));
        assert_eq!(params.get("post_id"), Some(&"99".to_string()));
    }

    #[test]
    fn test_open_router() {
        let handle = naml_net_http_server_open_router();
        assert!(handle > 0);

        let handle2 = naml_net_http_server_open_router();
        assert!(handle2 > handle);
    }
}
