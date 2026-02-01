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
//! - `read(path: string) -> string throws IOError`
//! - `read_bytes(path: string) -> bytes throws IOError`
//! - `write(path: string, content: string) throws IOError`
//! - `write_bytes(path: string, content: bytes) throws IOError`
//! - `append(path: string, content: string) throws IOError`
//! - `append_bytes(path: string, content: bytes) throws IOError`
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
//! Browser WASM uses OPFS (not yet implemented).
//!

use naml_std_core::{naml_exception_set, naml_string_new, NamlString};

/// Create a new IOError exception on the heap
///
/// Exception layout (matches naml exception codegen):
/// - Offset 0: message pointer (8 bytes)
/// - Offset 8: path pointer (8 bytes)
/// - Offset 16: code (8 bytes)
///
/// Total size: 24 bytes
#[unsafe(no_mangle)]
pub extern "C" fn naml_io_error_new(
    message: *const NamlString,
    path: *const NamlString,
    code: i64,
) -> *mut u8 {
    unsafe {
        // Allocate raw memory for exception (message + 2 fields = 24 bytes)
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate IOError");
        }

        // Store message at offset 0
        *(ptr as *mut i64) = message as i64;
        // Store path at offset 8
        *(ptr.add(8) as *mut i64) = path as i64;
        // Store code at offset 16
        *(ptr.add(16) as *mut i64) = code;

        ptr
    }
}

/// Create and throw an IOError from a Rust std::io::Error
///
/// This is a helper for fs functions to convert Rust errors to naml exceptions.
/// Returns null to indicate an exception was thrown.
fn throw_io_error(error: std::io::Error, path: &str) -> *mut u8 {
    let code = error.raw_os_error().unwrap_or(-1) as i64;
    let message = error.to_string();

    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let path_ptr = naml_string_new(path.as_ptr(), path.len());
        let io_error = naml_io_error_new(message_ptr, path_ptr, code);
        naml_exception_set(io_error);
    }

    std::ptr::null_mut()
}

/// Helper to extract path string from NamlString pointer
///
/// # Safety
/// The caller must ensure `s` is a valid pointer to a NamlString or null.
unsafe fn path_from_naml_string(s: *const NamlString) -> String {
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
