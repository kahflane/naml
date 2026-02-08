///
/// File Ownership and Identity Operations
///
/// Provides chown, lchown (change file ownership) and same_file (identity check).
/// Unix-only operations have Windows stubs that throw IOError.
///

use naml_std_core::NamlString;

use crate::{path_from_naml_string, throw_io_error};

/// Change file ownership (Unix only)
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chown(
    path: *const NamlString,
    uid: i64,
    gid: i64,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    let c_path = match std::ffi::CString::new(path_str.as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            let e = std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null byte");
            throw_io_error(e, &path_str);
            return 0;
        }
    };

    let result = unsafe { libc::chown(c_path.as_ptr(), uid as libc::uid_t, gid as libc::gid_t) };
    if result == 0 {
        0
    } else {
        throw_io_error(std::io::Error::last_os_error(), &path_str);
        0
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_chown(
    path: *const NamlString,
    _uid: i64,
    _gid: i64,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let e = std::io::Error::new(std::io::ErrorKind::Unsupported, "chown is not supported on this platform");
    throw_io_error(e, &path_str);
    0
}

/// Change symlink ownership without following (Unix only)
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_lchown(
    path: *const NamlString,
    uid: i64,
    gid: i64,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };

    let c_path = match std::ffi::CString::new(path_str.as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            let e = std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null byte");
            throw_io_error(e, &path_str);
            return 0;
        }
    };

    let result = unsafe { libc::lchown(c_path.as_ptr(), uid as libc::uid_t, gid as libc::gid_t) };
    if result == 0 {
        0
    } else {
        throw_io_error(std::io::Error::last_os_error(), &path_str);
        0
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_lchown(
    path: *const NamlString,
    _uid: i64,
    _gid: i64,
) -> i64 {
    let path_str = unsafe { path_from_naml_string(path) };
    let e = std::io::Error::new(std::io::ErrorKind::Unsupported, "lchown is not supported on this platform");
    throw_io_error(e, &path_str);
    0
}

/// Check if two paths refer to the same file (same device and inode)
/// Returns 1 if same file, 0 if different, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_same_file(
    path1: *const NamlString,
    path2: *const NamlString,
) -> i64 {
    use std::os::unix::fs::MetadataExt;

    let path1_str = unsafe { path_from_naml_string(path1) };
    let path2_str = unsafe { path_from_naml_string(path2) };

    let meta1 = match std::fs::metadata(&path1_str) {
        Ok(m) => m,
        Err(e) => {
            throw_io_error(e, &path1_str);
            return 0;
        }
    };

    let meta2 = match std::fs::metadata(&path2_str) {
        Ok(m) => m,
        Err(e) => {
            throw_io_error(e, &path2_str);
            return 0;
        }
    };

    if meta1.dev() == meta2.dev() && meta1.ino() == meta2.ino() { 1 } else { 0 }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_same_file(
    path1: *const NamlString,
    path2: *const NamlString,
) -> i64 {
    let path1_str = unsafe { path_from_naml_string(path1) };
    let path2_str = unsafe { path_from_naml_string(path2) };

    let abs1 = match std::fs::canonicalize(&path1_str) {
        Ok(p) => p,
        Err(e) => {
            throw_io_error(e, &path1_str);
            return 0;
        }
    };

    let abs2 = match std::fs::canonicalize(&path2_str) {
        Ok(p) => p,
        Err(e) => {
            throw_io_error(e, &path2_str);
            return 0;
        }
    };

    if abs1 == abs2 { 1 } else { 0 }
}
