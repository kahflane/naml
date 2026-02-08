//!
//! naml-std-fs - File System Operations
//!
//! Provides file system operations for naml programs.
//!
//! ## Exception
//!
//! All throwing functions use the `IOError` exception:
//! ```naml
//! exception IOError {
//!     message: string,
//!     path: string,
//!     code: int
//! }
//! ```
//!
//! ## Functions
//!
//! ### Basic File Operations
//! - `read(path: string) -> string throws IOError`
//! - `read_bytes(path: string) -> bytes throws IOError`
//! - `write(path: string, content: string) throws IOError`
//! - `write_bytes(path: string, content: bytes) throws IOError`
//! - `append(path: string, content: string) throws IOError`
//! - `append_bytes(path: string, content: bytes) throws IOError`
//!
//! ### File Handle Operations
//! - `file_open(path: string, mode: string) -> int throws IOError`
//! - `file_close(handle: int) throws IOError`
//! - `file_read(handle: int, count: int) -> string throws IOError`
//! - `file_read_line(handle: int) -> string throws IOError`
//! - `file_read_all(handle: int) -> string throws IOError`
//! - `file_write(handle: int, content: string) -> int throws IOError`
//! - `file_write_line(handle: int, content: string) -> int throws IOError`
//! - `file_flush(handle: int) throws IOError`
//! - `file_seek(handle: int, offset: int, whence: int) -> int throws IOError`
//! - `file_tell(handle: int) -> int throws IOError`
//! - `file_eof(handle: int) -> bool throws IOError`
//! - `file_size(handle: int) -> int throws IOError`
//!
//! ### Path Operations
//! - `exists(path: string) -> bool`
//! - `is_file(path: string) -> bool`
//! - `is_dir(path: string) -> bool`
//! - `list_dir(path: string) -> [string] throws IOError`
//! - `mkdir(path: string) throws IOError`
//! - `mkdir_all(path: string) throws IOError`
//! - `remove(path: string) throws IOError`
//! - `remove_all(path: string) throws IOError`
//! - `join(parts: [string]) -> string`
//! - `dirname(path: string) -> string`
//! - `basename(path: string) -> string`
//! - `extension(path: string) -> string`
//! - `absolute(path: string) -> string throws IOError`
//! - `size(path: string) -> int throws IOError`
//! - `modified(path: string) -> int throws IOError`
//! - `copy(src: string, dst: string) throws IOError`
//! - `rename(src: string, dst: string) throws IOError`
//!
//! ## Platform Support
//!
//! Native and Server WASM (uses std::fs).
//! Browser WASM uses OPFS (not yet implemented). TODO
//!

mod file_handle;
mod links;
mod mmap;
mod ownership;

pub use file_handle::*;
pub use links::*;
pub use mmap::*;
pub use ownership::*;

use naml_std_core::{
    naml_exception_set_typed, naml_stack_capture, naml_string_new,
    NamlBytes, NamlString,
    EXCEPTION_TYPE_IO_ERROR, EXCEPTION_TYPE_PERMISSION_ERROR,
};

/// Create a new IOError exception on the heap
///
/// Exception layout (matches naml exception codegen):
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes) - null, captured at throw time
/// - Offset 16: path pointer (8 bytes)
/// - Offset 24: code (8 bytes)
///
/// Total size: 32 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_io_error_new(
    message: *const NamlString,
    path: *const NamlString,
    code: i64,
) -> *mut u8 {
    unsafe {
        // Allocate raw memory for exception (message + stack + 2 fields = 32 bytes)
        let layout = std::alloc::Layout::from_size_align(32, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate IOError");
        }

        // Store message at offset 0
        *(ptr as *mut i64) = message as i64;
        // Store stack at offset 8 (null, captured at throw time from codegen)
        *(ptr.add(8) as *mut i64) = 0;
        // Store path at offset 16
        *(ptr.add(16) as *mut i64) = path as i64;
        // Store code at offset 24
        *(ptr.add(24) as *mut i64) = code;

        ptr
    }
}

/// Create a new PermissionError exception on the heap
///
/// Exception layout (matches naml exception codegen):
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: stack pointer (8 bytes) - null, captured at throw time
/// - Offset 16: path pointer (8 bytes)
/// - Offset 24: code (8 bytes)
///
/// Total size: 32 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_permission_error_new(
    message: *const NamlString,
    path: *const NamlString,
    code: i64,
) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(32, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate PermissionError");
        }

        *(ptr as *mut i64) = message as i64;
        *(ptr.add(8) as *mut i64) = 0;
        *(ptr.add(16) as *mut i64) = path as i64;
        *(ptr.add(24) as *mut i64) = code;

        ptr
    }
}

/// Check if an error is a permission error (EACCES or EPERM)
fn is_permission_error(error: &std::io::Error) -> bool {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => true,
        _ => {
            // Also check raw OS error codes
            if let Some(code) = error.raw_os_error() {
                // EACCES = 13, EPERM = 1 on Unix
                code == 13 || code == 1
            } else {
                false
            }
        }
    }
}

/// Create and throw a PermissionError from a Rust std::io::Error
pub(crate) fn throw_permission_error(error: std::io::Error, path: &str) -> *mut u8 {
    let code = error.raw_os_error().unwrap_or(-1) as i64;
    let message = error.to_string();

    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let path_ptr = naml_string_new(path.as_ptr(), path.len());
        let perm_error = naml_permission_error_new(message_ptr, path_ptr, code);

        let stack = naml_stack_capture();
        *(perm_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set_typed(perm_error, EXCEPTION_TYPE_PERMISSION_ERROR);
    }

    std::ptr::null_mut()
}

/// Create and throw an IOError from a Rust std::io::Error
///
/// This is a helper for fs functions to convert Rust errors to naml exceptions.
/// If the error is a permission error (EACCES/EPERM), throws PermissionError instead.
/// Returns null to indicate an exception was thrown.
pub(crate) fn throw_io_error(error: std::io::Error, path: &str) -> *mut u8 {
    // Check if this is a permission error
    if is_permission_error(&error) {
        return throw_permission_error(error, path);
    }

    let code = error.raw_os_error().unwrap_or(-1) as i64;
    let message = error.to_string();

    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let path_ptr = naml_string_new(path.as_ptr(), path.len());
        let io_error = naml_io_error_new(message_ptr, path_ptr, code);

        // Capture and store the stack trace at offset 8
        let stack = naml_stack_capture();
        *(io_error.add(8) as *mut *mut u8) = stack;

        naml_exception_set_typed(io_error, EXCEPTION_TYPE_IO_ERROR);
    }

    std::ptr::null_mut()
}

/// Helper to extract path string from NamlString pointer
///
/// # Safety
/// The caller must ensure `s` is a valid pointer to a NamlString or null.
pub(crate) unsafe fn path_from_naml_string(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

/// Read file contents as UTF-8 string
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_read(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::read_to_string(&path_str) {
        Ok(content) => unsafe { naml_string_new(content.as_ptr(), content.len()) },
        Err(e) => {
            throw_io_error(e, &path_str);
            std::ptr::null_mut()
        }
    }
}

/// Read file contents as raw bytes
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_read_bytes(path: *const NamlString) -> *mut naml_std_core::NamlArray {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::read(&path_str) {
        Ok(bytes) => {
            let arr = unsafe { naml_std_core::naml_array_new(bytes.len()) };
            for &byte in bytes.iter() {
                unsafe { naml_std_core::naml_array_push(arr, byte as i64) };
            }
            arr
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            std::ptr::null_mut()
        }
    }
}

/// Write string to file (overwrites existing content)
/// Returns 0 on success, sets exception and returns 0 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_write(path: *const NamlString, content: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let content_str = unsafe { path_from_naml_string(content) };

    match std::fs::write(&path_str, content_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Append string to file
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_append(path: *const NamlString, content: *const NamlString) -> i64 {
    use std::io::Write;

    let path_str = unsafe { path_from_naml_string(path) };
    let content_str = unsafe { path_from_naml_string(content) };

    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path_str)
        .and_then(|mut file| file.write_all(content_str.as_bytes()));

    match result {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Write bytes to file (overwrites existing content)
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_write_bytes(
    path: *const NamlString,
    content: *const NamlBytes,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    if content.is_null() {
        match std::fs::write(&path_str, &[]) {
            Ok(()) => return 0,
            Err(e) => {
                throw_io_error(e, &path_str);
                return 0;
            }
        }
    }

    // Extract bytes from NamlBytes
    let len = unsafe { (*content).len };
    let data = unsafe { std::slice::from_raw_parts((*content).data.as_ptr(), len) };

    match std::fs::write(&path_str, data) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Append bytes to file
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_append_bytes(
    path: *const NamlString,
    content: *const NamlBytes,
) -> i64 {
    use std::io::Write;

    let path_str = unsafe { path_from_naml_string(path) };

    if content.is_null() {
        return 0; // Nothing to append
    }

    // Extract bytes from NamlBytes
    let len = unsafe { (*content).len };
    let data = unsafe { std::slice::from_raw_parts((*content).data.as_ptr(), len) };

    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path_str)
        .and_then(|mut file| file.write_all(data));

    match result {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Check if path exists
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_exists(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    if std::path::Path::new(&path_str).exists() { 1 } else { 0 }
}

/// Check if path is a file
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_is_file(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    if std::path::Path::new(&path_str).is_file() { 1 } else { 0 }
}

/// Check if path is a directory
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_is_dir(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    if std::path::Path::new(&path_str).is_dir() { 1 } else { 0 }
}

/// Create a single directory
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mkdir(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::create_dir(&path_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Create directory and all parent directories
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mkdir_all(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::create_dir_all(&path_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Remove file or empty directory
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_remove(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let p = std::path::Path::new(&path_str);

    let result = if p.is_dir() {
        std::fs::remove_dir(&path_str)
    } else {
        std::fs::remove_file(&path_str)
    };

    match result {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Remove directory and all contents recursively
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_remove_all(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::remove_dir_all(&path_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Get parent directory of path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_dirname(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };
    let parent = std::path::Path::new(&path_str)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_string_new(parent.as_ptr(), parent.len()) }
}

/// Get filename from path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_basename(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };
    let name = std::path::Path::new(&path_str)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_string_new(name.as_ptr(), name.len()) }
}

/// Get file extension (without dot)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_extension(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };
    let ext = std::path::Path::new(&path_str)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_string_new(ext.as_ptr(), ext.len()) }
}

/// Convert to absolute path
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_absolute(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::canonicalize(&path_str) {
        Ok(abs) => {
            let abs_str = abs.to_string_lossy();
            unsafe { naml_string_new(abs_str.as_ptr(), abs_str.len()) }
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            std::ptr::null_mut()
        }
    }
}

/// Get file size in bytes
/// Returns -1 and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_size(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::metadata(&path_str) {
        Ok(meta) => meta.len() as i64,
        Err(e) => {
            throw_io_error(e, &path_str);
            -1
        }
    }
}

/// Get last modified time as Unix timestamp in milliseconds
/// Returns -1 and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_modified(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::metadata(&path_str).and_then(|m| m.modified()) {
        Ok(time) => {
            match time.duration_since(std::time::UNIX_EPOCH) {
                Ok(dur) => dur.as_millis() as i64,
                Err(_) => 0,
            }
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            -1
        }
    }
}

/// Copy file from src to dst
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_copy(src: *const NamlString, dst: *const NamlString) -> i64 {
    let src_str = unsafe { path_from_naml_string(src) };
    let dst_str = unsafe { path_from_naml_string(dst) };

    match std::fs::copy(&src_str, &dst_str) {
        Ok(_) => 0,
        Err(e) => {
            throw_io_error(e, &src_str);
            0
        }
    }
}

/// Rename/move file from src to dst
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_rename(src: *const NamlString, dst: *const NamlString) -> i64 {
    let src_str = unsafe { path_from_naml_string(src) };
    let dst_str = unsafe { path_from_naml_string(dst) };

    match std::fs::rename(&src_str, &dst_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &src_str);
            0
        }
    }
}

/// List directory contents
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_list_dir(path: *const NamlString) -> *mut naml_std_core::NamlArray {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::read_dir(&path_str) {
        Ok(entries) => {
            let entries: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path().to_string_lossy().into_owned())
                .collect();

            let arr = unsafe { naml_std_core::naml_array_new(entries.len()) };
            for entry in entries.iter() {
                let s = unsafe { naml_string_new(entry.as_ptr(), entry.len()) };
                unsafe { naml_std_core::naml_array_push(arr, s as i64) };
            }
            arr
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            std::ptr::null_mut()
        }
    }
}

/// Join path components with platform separator
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_join(parts: *const naml_std_core::NamlArray) -> *mut NamlString {
    if parts.is_null() {
        return unsafe { naml_string_new(std::ptr::null(), 0) };
    }

    let len = unsafe { naml_std_core::naml_array_len(parts) };
    let mut path = std::path::PathBuf::new();

    for i in 0..len {
        let part_ptr = unsafe { naml_std_core::naml_array_get(parts, i) as *const NamlString };
        if !part_ptr.is_null() {
            let part_str = unsafe { path_from_naml_string(part_ptr) };
            path.push(part_str);
        }
    }

    let result = path.to_string_lossy();
    unsafe { naml_string_new(result.as_ptr(), result.len()) }
}

/// Get current working directory
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_getwd() -> *mut NamlString {
    match std::env::current_dir() {
        Ok(path) => {
            let path_str = path.to_string_lossy();
            unsafe { naml_string_new(path_str.as_ptr(), path_str.len()) }
        }
        Err(e) => {
            throw_io_error(e, ".");
            std::ptr::null_mut()
        }
    }
}

/// Change current working directory
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chdir(path: *const NamlString) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::env::set_current_dir(&path_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Create a temporary file with optional prefix
/// Returns path to created file, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_create_temp(prefix: *const NamlString) -> *mut NamlString {
    let prefix_str = unsafe { path_from_naml_string(prefix) };
    let prefix_str = if prefix_str.is_empty() { "naml" } else { &prefix_str };

    match tempfile::Builder::new().prefix(prefix_str).tempfile() {
        Ok(file) => {
            let path = file.into_temp_path();
            let path_str = path.to_string_lossy();
            let result = unsafe { naml_string_new(path_str.as_ptr(), path_str.len()) };
            // Keep the file by not dropping TempPath
            std::mem::forget(path);
            result
        }
        Err(e) => {
            throw_io_error(e, prefix_str);
            std::ptr::null_mut()
        }
    }
}

/// Create a temporary directory with optional prefix
/// Returns path to created directory, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_mkdir_temp(prefix: *const NamlString) -> *mut NamlString {
    let prefix_str = unsafe { path_from_naml_string(prefix) };
    let prefix_str = if prefix_str.is_empty() { "naml" } else { &prefix_str };

    match tempfile::Builder::new().prefix(prefix_str).tempdir() {
        Ok(dir) => {
            let path_str = dir.path().to_string_lossy().into_owned();
            // Keep the directory by forgetting the TempDir (prevents cleanup)
            let _ = dir.keep();
            unsafe { naml_string_new(path_str.as_ptr(), path_str.len()) }
        }
        Err(e) => {
            throw_io_error(e, prefix_str);
            std::ptr::null_mut()
        }
    }
}

/// Change file permissions (Unix mode bits)
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chmod(path: *const NamlString, mode: i64) -> i64 {
    use std::os::unix::fs::PermissionsExt;

    let path_str = unsafe { path_from_naml_string(path) };

    let permissions = std::fs::Permissions::from_mode(mode as u32);
    match std::fs::set_permissions(&path_str, permissions) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Change file permissions (Windows - limited support)
#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chmod(path: *const NamlString, mode: i64) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    // On Windows, we can only toggle read-only
    let readonly = (mode & 0o200) == 0; // No write permission = readonly
    match std::fs::metadata(&path_str) {
        Ok(meta) => {
            let mut perms = meta.permissions();
            perms.set_readonly(readonly);
            match std::fs::set_permissions(&path_str, perms) {
                Ok(()) => 0,
                Err(e) => {
                    throw_io_error(e, &path_str);
                    0
                }
            }
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Truncate file to specified size
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_truncate(path: *const NamlString, size: i64) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    let file = match std::fs::OpenOptions::new().write(true).open(&path_str) {
        Ok(f) => f,
        Err(e) => {
            throw_io_error(e, &path_str);
            return 0;
        }
    };

    match file.set_len(size as u64) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

/// Convert std::fs::Metadata to a NamlArray with 7 elements:
/// [size, mode, modified, created, is_dir, is_file, is_symlink]
pub(crate) fn metadata_to_array(meta: &std::fs::Metadata) -> *mut naml_std_core::NamlArray {
    let arr = unsafe { naml_std_core::naml_array_new(7) };

    unsafe { naml_std_core::naml_array_push(arr, meta.len() as i64) };

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        unsafe { naml_std_core::naml_array_push(arr, meta.permissions().mode() as i64) };
    }
    #[cfg(not(unix))]
    {
        let mode = if meta.permissions().readonly() { 0o444 } else { 0o644 };
        unsafe { naml_std_core::naml_array_push(arr, mode) };
    }

    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    unsafe { naml_std_core::naml_array_push(arr, modified) };

    let created = meta
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    unsafe { naml_std_core::naml_array_push(arr, created) };

    unsafe { naml_std_core::naml_array_push(arr, if meta.is_dir() { 1 } else { 0 }) };
    unsafe { naml_std_core::naml_array_push(arr, if meta.is_file() { 1 } else { 0 }) };
    unsafe { naml_std_core::naml_array_push(arr, if meta.is_symlink() { 1 } else { 0 }) };

    arr
}

/// Get file metadata (stat)
/// Returns an array with: [size, mode, modified, created, is_dir, is_file, is_symlink]
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_stat(path: *const NamlString) -> *mut naml_std_core::NamlArray {
    let path_str = unsafe { path_from_naml_string(path) };

    let meta = match std::fs::metadata(&path_str) {
        Ok(m) => m,
        Err(e) => {
            throw_io_error(e, &path_str);
            return std::ptr::null_mut();
        }
    };

    metadata_to_array(&meta)
}

/// Change file access and modification times
/// atime_ms and mtime_ms are Unix timestamps in milliseconds
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chtimes(
    path: *const NamlString,
    atime_ms: i64,
    mtime_ms: i64,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    let file = match std::fs::OpenOptions::new().write(true).open(&path_str) {
        Ok(f) => f,
        Err(e) => {
            throw_io_error(e, &path_str);
            return 0;
        }
    };

    let atime = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_millis(atime_ms as u64);
    let mtime = std::time::SystemTime::UNIX_EPOCH
        + std::time::Duration::from_millis(mtime_ms as u64);

    let times = std::fs::FileTimes::new()
        .set_accessed(atime)
        .set_modified(mtime);

    match file.set_times(times) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &path_str);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_operations() {
        unsafe {
            let path = naml_string_new(b"/foo/bar/baz.txt".as_ptr(), 16);

            let dirname = naml_fs_dirname(path);
            assert!(!dirname.is_null());

            let basename = naml_fs_basename(path);
            assert!(!basename.is_null());

            let ext = naml_fs_extension(path);
            assert!(!ext.is_null());
        }
    }
}
