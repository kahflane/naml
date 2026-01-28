//!
//! naml Runtime
//!
//! This module provides the runtime support for naml programs compiled with
//! Cranelift JIT. It includes:
//!
//! - Value representation (tagged union for dynamic typing at runtime boundaries)
//! - Reference-counted memory management
//! - Array operations
//! - String operations
//! - Struct field access
//! - Exception handling support
//!
//! Design: All heap objects use atomic reference counting for thread safety.
//! Values are passed as 64-bit tagged pointers or inline primitives.
//!

pub mod value;
pub mod array;
pub mod scheduler;
pub mod channel;
pub mod map;
pub mod bytes;

pub use value::*;
pub use array::*;
pub use scheduler::*;
pub use channel::*;
pub use map::*;
pub use bytes::*;

use std::cell::Cell;
use std::io::Write;

thread_local! {
    static CURRENT_EXCEPTION: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) };
}

/// Initialize the runtime (call once at program start)
pub fn init() {
    // Ensure stdout is line-buffered for print statements
    let _ = std::io::stdout().flush();
}

/// Set the current exception (called by throw)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_set(exception_ptr: *mut u8) {
    CURRENT_EXCEPTION.with(|ex| ex.set(exception_ptr));
}

/// Get the current exception pointer (0 if none)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_get() -> *mut u8 {
    CURRENT_EXCEPTION.with(|ex| ex.get())
}

/// Clear the current exception (called after catch handles it)
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_clear() {
    CURRENT_EXCEPTION.with(|ex| ex.set(std::ptr::null_mut()));
}

use std::sync::atomic::{AtomicU64, Ordering};

static RNG_STATE: AtomicU64 = AtomicU64::new(0);

fn rng_next() -> u64 {
    let mut s = RNG_STATE.load(Ordering::Relaxed);
    if s == 0 {
        s = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        if s == 0 { s = 1; }
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    RNG_STATE.store(s, Ordering::Relaxed);
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_random(min: i64, max: i64) -> i64 {
    if min >= max {
        return min;
    }
    let range = (max - min + 1) as u64;
    let r = rng_next() % range;
    min + r as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_random_float() -> f64 {
    let r = rng_next();
    (r >> 11) as f64 / (1u64 << 53) as f64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_read_line() -> *mut value::NamlString {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    if input.ends_with('\n') { input.pop(); }
    if input.ends_with('\r') { input.pop(); }
    let cstr = std::ffi::CString::new(input).unwrap_or_default();
    unsafe { value::naml_string_from_cstr(cstr.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_read_key() -> i64 {
    use std::os::unix::io::AsRawFd;

    let stdin_fd = std::io::stdin().as_raw_fd();

    unsafe {
        let mut old_termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(stdin_fd, &mut old_termios) != 0 {
            return -1;
        }

        let mut raw = old_termios;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO);
        raw.c_cc[libc::VMIN] = 0;
        raw.c_cc[libc::VTIME] = 0;

        if libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw) != 0 {
            return -1;
        }

        let mut buf: [u8; 1] = [0];
        let n = libc::read(stdin_fd, buf.as_mut_ptr() as *mut libc::c_void, 1);

        libc::tcsetattr(stdin_fd, libc::TCSANOW, &old_termios);

        if n <= 0 { -1 } else { buf[0] as i64 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_warn(s: *const value::NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(msg) = std::str::from_utf8(slice) {
                eprintln!("warning: {}", msg);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_error(s: *const value::NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(msg) = std::str::from_utf8(slice) {
                eprintln!("error: {}", msg);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_panic(s: *const value::NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(msg) = std::str::from_utf8(slice) {
                eprintln!("panic: {}", msg);
            }
        }
    }
    std::process::abort();
}

/// Check if there's a pending exception
#[unsafe(no_mangle)]
pub extern "C" fn naml_exception_check() -> i64 {
    CURRENT_EXCEPTION.with(|ex| if ex.get().is_null() { 0 } else { 1 })
}
