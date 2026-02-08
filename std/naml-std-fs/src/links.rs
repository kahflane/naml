///
/// Link and Symlink Operations
///
/// Provides symbolic link, hard link, and link metadata operations.
/// Extracted from lib.rs to keep file sizes under 1000 lines.
///
/// Functions:
/// - `symlink(target, link_path)` - Create a symbolic link
/// - `readlink(path) -> string` - Read the target of a symbolic link
/// - `lstat(path) -> [int]` - Get metadata without following symlinks
/// - `link(src, dst)` - Create a hard link
///

use naml_std_core::{naml_string_new, NamlString};

use crate::{path_from_naml_string, throw_io_error};

/// Create a symbolic link
/// Returns 0 on success, sets exception on error
#[cfg(unix)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_symlink(
    target: *const NamlString,
    link_path: *const NamlString,
) -> i64 {
    let target_str = unsafe { path_from_naml_string(target) };
    let link_str = unsafe { path_from_naml_string(link_path) };

    match std::os::unix::fs::symlink(&target_str, &link_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &link_str);
            0
        }
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_symlink(
    target: *const NamlString,
    link_path: *const NamlString,
) -> i64 {
    let target_str = unsafe { path_from_naml_string(target) };
    let link_str = unsafe { path_from_naml_string(link_path) };

    match std::os::windows::fs::symlink_file(&target_str, &link_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &link_str);
            0
        }
    }
}

/// Read the target of a symbolic link
/// Returns the target path string, or null on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_readlink(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { path_from_naml_string(path) };

    match std::fs::read_link(&path_str) {
        Ok(target) => {
            let target_str = target.to_string_lossy();
            unsafe { naml_string_new(target_str.as_ptr(), target_str.len()) }
        }
        Err(e) => {
            throw_io_error(e, &path_str);
            std::ptr::null_mut()
        }
    }
}

/// Get metadata without following symlinks (lstat)
/// Returns an array with: [size, mode, modified, created, is_dir, is_file, is_symlink]
/// Same layout as naml_fs_stat but uses symlink_metadata
/// Returns null and sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_lstat(path: *const NamlString) -> *mut naml_std_core::NamlArray {
    let path_str = unsafe { path_from_naml_string(path) };

    let meta = match std::fs::symlink_metadata(&path_str) {
        Ok(m) => m,
        Err(e) => {
            throw_io_error(e, &path_str);
            return std::ptr::null_mut();
        }
    };

    crate::metadata_to_array(&meta)
}

/// Create a hard link
/// Returns 0 on success, sets exception on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_fs_link(
    src: *const NamlString,
    dst: *const NamlString,
) -> i64 {
    let src_str = unsafe { path_from_naml_string(src) };
    let dst_str = unsafe { path_from_naml_string(dst) };

    match std::fs::hard_link(&src_str, &dst_str) {
        Ok(()) => 0,
        Err(e) => {
            throw_io_error(e, &src_str);
            0
        }
    }
}
