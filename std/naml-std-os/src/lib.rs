///
/// naml-std-os - Operating System Information
///
/// Provides OS-level system information and Unix user identity functions.
///
/// ## System Information (Issue #134)
///
/// - `hostname() -> string throws OSError` - Get machine hostname
/// - `temp_dir() -> string` - Get temp directory path
/// - `home_dir() -> string throws OSError` - Get user home directory
/// - `cache_dir() -> string throws OSError` - Get user cache directory
/// - `config_dir() -> string throws OSError` - Get user config directory
/// - `executable() -> string throws OSError` - Get current executable path
/// - `pagesize() -> int` - Get system page size
///
/// ## User/Group Identity (Issue #133, Unix-only)
///
/// - `getuid() -> int` - Get real user ID
/// - `geteuid() -> int` - Get effective user ID
/// - `getgid() -> int` - Get real group ID
/// - `getegid() -> int` - Get effective group ID
/// - `getgroups() -> [int] throws OSError` - Get supplementary group list
///
/// ## Platform Notes
///
/// System information functions work cross-platform via Rust's std library.
/// User/group functions use libc and return -1 on non-Unix platforms.
/// Directory functions resolve platform-specific well-known paths:
///   - macOS: ~/Library/Caches, ~/Library/Application Support
///   - Linux: ~/.cache, ~/.config (XDG_* respected)
///   - Windows: %LOCALAPPDATA%, %APPDATA%
///

use naml_std_core::{
    naml_array_new, naml_array_push, naml_exception_set_typed, naml_stack_capture,
    naml_string_new, naml_struct_new, naml_struct_set_field, NamlArray, NamlString, NamlStruct,
    EXCEPTION_TYPE_OS_ERROR,
};

const OS_ERROR_STRUCT_TYPE_ID: u32 = 0xFFFF_0008;

unsafe fn naml_from_string(s: &str) -> *mut NamlString {
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_os_error_new(
    message: *const NamlString,
    code: i64,
) -> *mut NamlStruct {
    unsafe {
        let exc = naml_struct_new(OS_ERROR_STRUCT_TYPE_ID, 2);
        naml_struct_set_field(exc, 0, message as i64);
        naml_struct_set_field(exc, 1, code);
        exc
    }
}

fn throw_os_error(message: &str, code: i32) {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let exc = naml_os_error_new(message_ptr, code as i64);

        let stack = naml_stack_capture();
        *(exc as *mut u8).add(8).cast::<*mut u8>() = stack;

        naml_exception_set_typed(exc as *mut u8, EXCEPTION_TYPE_OS_ERROR);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_hostname() -> *mut NamlString {
    let mut buf = [0u8; 256];
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if rc != 0 {
        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
        throw_os_error("failed to get hostname", errno);
        return unsafe { naml_from_string("") };
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let name = std::str::from_utf8(&buf[..len]).unwrap_or("");
    unsafe { naml_from_string(name) }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_temp_dir() -> *mut NamlString {
    let path = std::env::temp_dir();
    let s = path.to_string_lossy();
    unsafe { naml_from_string(&s) }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_home_dir() -> *mut NamlString {
    match std::env::var("HOME") {
        Ok(home) => unsafe { naml_from_string(&home) },
        Err(_) => {
            throw_os_error("HOME environment variable not set", -1);
            unsafe { naml_from_string("") }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_cache_dir() -> *mut NamlString {
    #[cfg(target_os = "macos")]
    {
        match std::env::var("HOME") {
            Ok(home) => {
                let path = format!("{}/Library/Caches", home);
                return unsafe { naml_from_string(&path) };
            }
            Err(_) => {
                throw_os_error("cannot determine cache directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
            return unsafe { naml_from_string(&xdg) };
        }
        match std::env::var("HOME") {
            Ok(home) => {
                let path = format!("{}/.cache", home);
                return unsafe { naml_from_string(&path) };
            }
            Err(_) => {
                throw_os_error("cannot determine cache directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        match std::env::var("LOCALAPPDATA") {
            Ok(dir) => return unsafe { naml_from_string(&dir) },
            Err(_) => {
                throw_os_error("cannot determine cache directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        throw_os_error("cache_dir not supported on this platform", -1);
        unsafe { naml_from_string("") }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_config_dir() -> *mut NamlString {
    #[cfg(target_os = "macos")]
    {
        match std::env::var("HOME") {
            Ok(home) => {
                let path = format!("{}/Library/Application Support", home);
                return unsafe { naml_from_string(&path) };
            }
            Err(_) => {
                throw_os_error("cannot determine config directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return unsafe { naml_from_string(&xdg) };
        }
        match std::env::var("HOME") {
            Ok(home) => {
                let path = format!("{}/.config", home);
                return unsafe { naml_from_string(&path) };
            }
            Err(_) => {
                throw_os_error("cannot determine config directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        match std::env::var("APPDATA") {
            Ok(dir) => return unsafe { naml_from_string(&dir) },
            Err(_) => {
                throw_os_error("cannot determine config directory", -1);
                return unsafe { naml_from_string("") };
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        throw_os_error("config_dir not supported on this platform", -1);
        unsafe { naml_from_string("") }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_executable() -> *mut NamlString {
    match std::env::current_exe() {
        Ok(path) => {
            let s = path.to_string_lossy();
            unsafe { naml_from_string(&s) }
        }
        Err(e) => {
            let msg = format!("failed to get executable path: {}", e);
            throw_os_error(&msg, -1);
            unsafe { naml_from_string("") }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_pagesize() -> i64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_getuid() -> i64 {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() as i64 }
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_geteuid() -> i64 {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() as i64 }
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_getgid() -> i64 {
    #[cfg(unix)]
    {
        unsafe { libc::getgid() as i64 }
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_getegid() -> i64 {
    #[cfg(unix)]
    {
        unsafe { libc::getegid() as i64 }
    }
    #[cfg(not(unix))]
    {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_os_getgroups() -> *mut NamlArray {
    #[cfg(unix)]
    {
        let mut ngroups = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        if ngroups < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            throw_os_error("failed to get supplementary groups", errno);
            return unsafe { naml_array_new(0) };
        }

        let mut groups = vec![0 as libc::gid_t; ngroups as usize];
        ngroups = unsafe { libc::getgroups(ngroups, groups.as_mut_ptr()) };
        if ngroups < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            throw_os_error("failed to get supplementary groups", errno);
            return unsafe { naml_array_new(0) };
        }

        let arr = unsafe { naml_array_new(ngroups as usize) };
        for i in 0..ngroups as usize {
            unsafe { naml_array_push(arr, groups[i] as i64) };
        }
        arr
    }
    #[cfg(not(unix))]
    {
        throw_os_error("getgroups not supported on this platform", -1);
        unsafe { naml_array_new(0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hostname() {
        let result = naml_os_hostname();
        assert!(!result.is_null());
        let name = unsafe {
            let slice = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            String::from_utf8_lossy(slice).into_owned()
        };
        assert!(!name.is_empty());
    }

    #[test]
    fn test_temp_dir() {
        let result = naml_os_temp_dir();
        assert!(!result.is_null());
        let path = unsafe {
            let slice = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            String::from_utf8_lossy(slice).into_owned()
        };
        assert!(!path.is_empty());
    }

    #[test]
    fn test_pagesize() {
        let size = naml_os_pagesize();
        assert!(size > 0);
        assert!(size % 1024 == 0 || size == 512);
    }

    #[test]
    #[cfg(unix)]
    fn test_getuid() {
        let uid = naml_os_getuid();
        assert!(uid >= 0);
    }

    #[test]
    #[cfg(unix)]
    fn test_getgid() {
        let gid = naml_os_getgid();
        assert!(gid >= 0);
    }
}
