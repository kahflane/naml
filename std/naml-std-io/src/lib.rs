//!
//! naml-std-io - Input/Output Operations
//!
//! Provides terminal I/O and console control for naml programs.
//!
//! ## Functions
//!
//! - `read_key() -> int` - Non-blocking single key read (-1 if no key)
//! - `read_line() -> string` - Read a line from stdin (blocking)
//! - `clear_screen()` - Clear the terminal screen
//! - `set_cursor(x: int, y: int)` - Move cursor to position (0-indexed)
//! - `hide_cursor()` - Hide the terminal cursor
//! - `show_cursor()` - Show the terminal cursor
//! - `terminal_width() -> int` - Get terminal width in columns
//! - `terminal_height() -> int` - Get terminal height in rows
//!
//! ## Platform Support
//!
//! Currently supports Unix-like systems (Linux, macOS) only.
//! Uses ANSI escape codes for terminal control and libc for terminal queries.
//!

use std::io::Write;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// Read a single key without blocking
/// Returns the key code or -1 if no key is available
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn naml_read_key() -> i64 {
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

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn naml_read_key() -> i64 {
    -1
}

/// Read a line from stdin (blocking)
#[unsafe(no_mangle)]
pub extern "C" fn naml_read_line() -> *mut naml_std_core::NamlString {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    if input.ends_with('\n') { input.pop(); }
    if input.ends_with('\r') { input.pop(); }
    let cstr = std::ffi::CString::new(input).unwrap_or_default();
    unsafe { naml_std_core::naml_string_from_cstr(cstr.as_ptr()) }
}

/// Clear the terminal screen and move cursor to top-left
#[unsafe(no_mangle)]
pub extern "C" fn naml_clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = std::io::stdout().flush();
}

/// Move cursor to position (x, y) where (0, 0) is top-left
#[unsafe(no_mangle)]
pub extern "C" fn naml_set_cursor(x: i64, y: i64) {
    print!("\x1b[{};{}H", y + 1, x + 1);
    let _ = std::io::stdout().flush();
}

/// Hide the terminal cursor
#[unsafe(no_mangle)]
pub extern "C" fn naml_hide_cursor() {
    print!("\x1b[?25l");
    let _ = std::io::stdout().flush();
}

/// Show the terminal cursor
#[unsafe(no_mangle)]
pub extern "C" fn naml_show_cursor() {
    print!("\x1b[?25h");
    let _ = std::io::stdout().flush();
}

/// Get terminal width in columns
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn naml_terminal_width() -> i64 {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) == 0 {
            ws.ws_col as i64
        } else {
            80
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn naml_terminal_width() -> i64 {
    80
}

/// Get terminal height in rows
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn naml_terminal_height() -> i64 {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) == 0 {
            ws.ws_row as i64
        } else {
            24
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn naml_terminal_height() -> i64 {
    24
}

/// Print a warning message to stderr
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_warn(s: *const naml_std_core::NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(msg) = std::str::from_utf8(slice) {
                eprintln!("warning: {}", msg);
            }
        }
    }
}

/// Print an error message to stderr
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_error(s: *const naml_std_core::NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(msg) = std::str::from_utf8(slice) {
                eprintln!("error: {}", msg);
            }
        }
    }
}

/// Print a panic message to stderr and abort
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_panic(s: *const naml_std_core::NamlString) {
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
