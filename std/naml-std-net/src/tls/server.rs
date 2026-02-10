///
/// TLS Server Implementation
///
/// Provides TLS server functions for naml programs. Wraps TCP listeners
/// with TLS configuration for accepting encrypted connections.
///
/// ## Functions
///
/// - `naml_net_tls_server_wrap_listener` - Wrap TCP listener with TLS config
/// - `naml_net_tls_server_accept` - Accept a TLS connection
/// - `naml_net_tls_server_close_listener` - Close TLS listener
///

use std::collections::HashMap;
use std::io::BufReader;
use std::sync::{Arc, Mutex, OnceLock};

use rustls::pki_types::PrivateKeyDer;
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use naml_std_core::NamlString;

use crate::errors::{string_from_naml, throw_network_error, throw_tls_error};
use crate::tcp::server::{get_listeners, next_handle};

use super::{TlsStream, get_tls_streams};

struct TlsListenerConfig {
    tcp_listener_handle: i64,
    server_config: Arc<ServerConfig>,
}

static TLS_LISTENERS: OnceLock<Mutex<HashMap<i64, TlsListenerConfig>>> = OnceLock::new();

fn get_tls_listeners() -> &'static Mutex<HashMap<i64, TlsListenerConfig>> {
    TLS_LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_certs_and_key(
    cert_path: &str,
    key_path: &str,
) -> Result<Arc<ServerConfig>, String> {
    let cert_file = std::fs::File::open(cert_path)
        .map_err(|e| format!("failed to open certificate file '{}': {}", cert_path, e))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed to parse certificates: {}", e))?;

    if certs.is_empty() {
        return Err("no certificates found in certificate file".to_string());
    }

    let key_file = std::fs::File::open(key_path)
        .map_err(|e| format!("failed to open key file '{}': {}", key_path, e))?;
    let mut key_reader = BufReader::new(key_file);

    let key: PrivateKeyDer = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| format!("failed to parse private key: {}", e))?
        .ok_or_else(|| "no private key found in key file".to_string())?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("invalid TLS server config: {}", e))?;

    Ok(Arc::new(config))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_net_tls_server_wrap_listener(
    listener_handle: i64,
    cert_path: *const NamlString,
    key_path: *const NamlString,
) -> i64 {
    let cert_str = unsafe { string_from_naml(cert_path) };
    let key_str = unsafe { string_from_naml(key_path) };

    {
        let listeners = get_listeners().lock().unwrap();
        if !listeners.contains_key(&listener_handle) {
            drop(listeners);
            throw_network_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid TCP listener handle",
            ));
            return -1;
        }
    }

    let server_config = match load_certs_and_key(&cert_str, &key_str) {
        Ok(config) => config,
        Err(msg) => {
            throw_tls_error(&msg);
            return -1;
        }
    };

    let handle = next_handle();
    get_tls_listeners().lock().unwrap().insert(
        handle,
        TlsListenerConfig {
            tcp_listener_handle: listener_handle,
            server_config,
        },
    );
    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_server_accept(tls_listener_handle: i64) -> i64 {
    let (tcp_listener_handle, server_config) = {
        let tls_listeners = get_tls_listeners().lock().unwrap();
        match tls_listeners.get(&tls_listener_handle) {
            Some(config) => (config.tcp_listener_handle, Arc::clone(&config.server_config)),
            None => {
                drop(tls_listeners);
                throw_network_error(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Invalid TLS listener handle",
                ));
                return -1;
            }
        }
    };

    let tcp_stream = {
        let listeners = get_listeners().lock().unwrap();
        let listener = match listeners.get(&tcp_listener_handle) {
            Some(l) => l,
            None => {
                drop(listeners);
                throw_network_error(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Underlying TCP listener not found",
                ));
                return -1;
            }
        };
        let listener_clone = match listener.try_clone() {
            Ok(l) => l,
            Err(e) => {
                drop(listeners);
                throw_network_error(e);
                return -1;
            }
        };
        drop(listeners);

        match listener_clone.accept() {
            Ok((stream, _)) => stream,
            Err(e) => {
                throw_network_error(e);
                return -1;
            }
        }
    };

    let conn = match ServerConnection::new(server_config) {
        Ok(c) => c,
        Err(e) => {
            throw_tls_error(&format!("TLS server handshake failed: {}", e));
            return -1;
        }
    };

    let tls_stream = StreamOwned::new(conn, tcp_stream);
    let handle = next_handle();
    get_tls_streams()
        .lock()
        .unwrap()
        .insert(handle, TlsStream::Server(tls_stream));
    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_net_tls_server_close_listener(handle: i64) {
    get_tls_listeners().lock().unwrap().remove(&handle);
}
