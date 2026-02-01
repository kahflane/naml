///
/// naml-std-path - Cross-platform path manipulation
///
/// Provides path operations using Rust's std::path for cross-platform
/// compatibility across Windows, Unix, and WASM targets.
///
/// ## Functions
///
/// - `join(parts: [string]) -> string` - Join path components
/// - `normalize(path: string) -> string` - Normalize path (resolve . and ..)
/// - `is_absolute(path: string) -> bool` - Check if path is absolute
/// - `is_relative(path: string) -> bool` - Check if path is relative
/// - `dirname(path: string) -> string` - Get parent directory
/// - `basename(path: string) -> string` - Get filename
/// - `extension(path: string) -> string` - Get extension (without dot)
/// - `stem(path: string) -> string` - Get filename without extension
/// - `with_extension(path: string, ext: string) -> string` - Change extension
/// - `components(path: string) -> [string]` - Split into components
/// - `separator() -> string` - Get platform path separator
/// - `to_slash(path: string) -> string` - Convert to forward slashes
/// - `from_slash(path: string) -> string` - Convert from forward slashes
///

use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};

use naml_std_core::{naml_array_new, naml_array_push, naml_string_new, NamlArray, NamlString};

/// Helper to extract string from NamlString pointer
unsafe fn string_from_naml(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

/// Helper to create NamlString from Rust string
unsafe fn naml_from_string(s: &str) -> *mut NamlString {
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

/// Join path components with platform separator
/// join(["a", "b", "c"]) -> "a/b/c" (Unix) or "a\\b\\c" (Windows)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_join(parts: *const NamlArray) -> *mut NamlString {
    if parts.is_null() {
        return unsafe { naml_string_new(std::ptr::null(), 0) };
    }

    let len = unsafe { naml_std_core::naml_array_len(parts) };
    let mut path = PathBuf::new();

    for i in 0..len {
        let part_ptr = unsafe { naml_std_core::naml_array_get(parts, i) as *const NamlString };
        if !part_ptr.is_null() {
            let part_str = unsafe { string_from_naml(part_ptr) };
            path.push(part_str);
        }
    }

    let result = path.to_string_lossy();
    unsafe { naml_from_string(&result) }
}

/// Normalize path by resolving . and .. components
/// normalize("a/b/../c/./d") -> "a/c/d"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_normalize(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let path = Path::new(&path_str);

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(p) => normalized.push(p.as_os_str()),
            Component::RootDir => normalized.push(Component::RootDir.as_os_str()),
            Component::CurDir => {} // Skip .
            Component::ParentDir => {
                // Go up one level if possible
                if !normalized.pop() {
                    // If we can't pop, keep the ..
                    normalized.push("..");
                }
            }
            Component::Normal(c) => normalized.push(c),
        }
    }

    // Handle empty result
    if normalized.as_os_str().is_empty() {
        return unsafe { naml_from_string(".") };
    }

    let result = normalized.to_string_lossy();
    unsafe { naml_from_string(&result) }
}

/// Check if path is absolute
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_is_absolute(path: *const NamlString) -> i64 {
    let path_str = unsafe { string_from_naml(path) };
    if Path::new(&path_str).is_absolute() {
        1
    } else {
        0
    }
}

/// Check if path is relative
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_is_relative(path: *const NamlString) -> i64 {
    let path_str = unsafe { string_from_naml(path) };
    if Path::new(&path_str).is_relative() {
        1
    } else {
        0
    }
}

/// Get parent directory
/// dirname("/a/b/c") -> "/a/b"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_dirname(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let parent = Path::new(&path_str)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_from_string(&parent) }
}

/// Get filename (basename)
/// basename("/a/b/c.txt") -> "c.txt"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_basename(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let name = Path::new(&path_str)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_from_string(&name) }
}

/// Get file extension (without dot)
/// extension("/a/b/c.txt") -> "txt"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_extension(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let ext = Path::new(&path_str)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_from_string(&ext) }
}

/// Get filename without extension (stem)
/// stem("/a/b/c.txt") -> "c"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_stem(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let stem = Path::new(&path_str)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    unsafe { naml_from_string(&stem) }
}

/// Change file extension
/// with_extension("/a/b/c.txt", "md") -> "/a/b/c.md"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_with_extension(
    path: *const NamlString,
    ext: *const NamlString,
) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let ext_str = unsafe { string_from_naml(ext) };

    let mut path_buf = PathBuf::from(&path_str);
    path_buf.set_extension(&ext_str);

    let result = path_buf.to_string_lossy();
    unsafe { naml_from_string(&result) }
}

/// Split path into components
/// components("/a/b/c") -> ["/", "a", "b", "c"] (Unix)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_components(path: *const NamlString) -> *mut NamlArray {
    let path_str = unsafe { string_from_naml(path) };
    let path = Path::new(&path_str);

    let components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    let arr = unsafe { naml_array_new(components.len()) };
    for component in components.iter() {
        let s = unsafe { naml_from_string(component) };
        unsafe { naml_array_push(arr, s as i64) };
    }
    arr
}

/// Get platform path separator
/// separator() -> "/" (Unix) or "\\" (Windows)
#[unsafe(no_mangle)]
pub extern "C" fn naml_path_separator() -> *mut NamlString {
    let sep = MAIN_SEPARATOR.to_string();
    unsafe { naml_from_string(&sep) }
}

/// Convert path to use forward slashes (for URLs, cross-platform storage)
/// to_slash("a\\b\\c") -> "a/b/c"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_to_slash(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let result = path_str.replace('\\', "/");
    unsafe { naml_from_string(&result) }
}

/// Convert path from forward slashes to platform-specific separator
/// from_slash("a/b/c") -> "a\\b\\c" (Windows) or "a/b/c" (Unix)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_from_slash(path: *const NamlString) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };

    #[cfg(windows)]
    let result = path_str.replace('/', "\\");

    #[cfg(not(windows))]
    let result = path_str;

    unsafe { naml_from_string(&result) }
}

/// Check if path has a root (starts with / on Unix, or C:\ on Windows)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_has_root(path: *const NamlString) -> i64 {
    let path_str = unsafe { string_from_naml(path) };
    if Path::new(&path_str).has_root() {
        1
    } else {
        0
    }
}

/// Check if path starts with another path
/// starts_with("/a/b/c", "/a/b") -> true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_starts_with(
    path: *const NamlString,
    prefix: *const NamlString,
) -> i64 {
    let path_str = unsafe { string_from_naml(path) };
    let prefix_str = unsafe { string_from_naml(prefix) };

    if Path::new(&path_str).starts_with(&prefix_str) {
        1
    } else {
        0
    }
}

/// Check if path ends with another path
/// ends_with("/a/b/c", "b/c") -> true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_ends_with(
    path: *const NamlString,
    suffix: *const NamlString,
) -> i64 {
    let path_str = unsafe { string_from_naml(path) };
    let suffix_str = unsafe { string_from_naml(suffix) };

    if Path::new(&path_str).ends_with(&suffix_str) {
        1
    } else {
        0
    }
}

/// Strip prefix from path
/// strip_prefix("/a/b/c", "/a") -> "b/c"
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_strip_prefix(
    path: *const NamlString,
    prefix: *const NamlString,
) -> *mut NamlString {
    let path_str = unsafe { string_from_naml(path) };
    let prefix_str = unsafe { string_from_naml(prefix) };

    let result = Path::new(&path_str)
        .strip_prefix(&prefix_str)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or(path_str);

    unsafe { naml_from_string(&result) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        unsafe {
            let path = naml_string_new(b"a/b/../c/./d".as_ptr(), 12);
            let result = naml_path_normalize(path);
            let result_str = string_from_naml(result);
            assert_eq!(result_str, "a/c/d");
        }
    }

    #[test]
    fn test_is_absolute() {
        unsafe {
            let abs_path = naml_string_new(b"/absolute/path".as_ptr(), 14);
            let rel_path = naml_string_new(b"relative/path".as_ptr(), 13);

            assert_eq!(naml_path_is_absolute(abs_path), 1);
            assert_eq!(naml_path_is_absolute(rel_path), 0);
        }
    }

    #[test]
    fn test_stem() {
        unsafe {
            let path = naml_string_new(b"/a/b/file.txt".as_ptr(), 13);
            let result = naml_path_stem(path);
            let result_str = string_from_naml(result);
            assert_eq!(result_str, "file");
        }
    }

    #[test]
    fn test_to_slash() {
        unsafe {
            let path = naml_string_new(b"a\\b\\c".as_ptr(), 5);
            let result = naml_path_to_slash(path);
            let result_str = string_from_naml(result);
            assert_eq!(result_str, "a/b/c");
        }
    }
}
