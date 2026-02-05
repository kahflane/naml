///
/// naml-std-env - Environment Variable Operations
///
/// Provides environment variable access for naml programs.
///
/// ## Functions
///
/// - `getenv(key: string) -> string` - Get env var, empty if not set
/// - `lookup_env(key: string) -> option<string>` - Get env var with exists check
/// - `setenv(key: string, value: string) throws EnvError` - Set env var
/// - `unsetenv(key: string) throws EnvError` - Remove env var
/// - `clearenv() throws EnvError` - Clear all env vars
/// - `environ() -> [string]` - Get all env vars as "KEY=VALUE" array
/// - `expand_env(s: string) -> string` - Expand $VAR and ${VAR} in string
///
/// ## Platform Notes
///
/// All functions work cross-platform via Rust's std::env.
/// `clearenv` iterates and removes all vars since libc::clearenv
/// is not portable.
///

use naml_std_core::{
    naml_array_new, naml_array_push, naml_exception_set_typed, naml_stack_capture,
    naml_string_new, naml_struct_new, naml_struct_set_field, NamlArray, NamlString, NamlStruct,
    EXCEPTION_TYPE_ENV_ERROR,
};
const ENV_ERROR_STRUCT_TYPE_ID: u32 = 0xFFFF_0007;

unsafe fn string_from_naml(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

unsafe fn naml_from_string(s: &str) -> *mut NamlString {
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_error_new(
    message: *const NamlString,
    key: *const NamlString,
) -> *mut NamlStruct {
    unsafe {
        let exc = naml_struct_new(ENV_ERROR_STRUCT_TYPE_ID, 2);
        naml_struct_set_field(exc, 0, message as i64);
        naml_struct_set_field(exc, 1, key as i64);
        exc
    }
}

fn throw_env_error(message: &str, key: &str) {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let key_ptr = naml_string_new(key.as_ptr(), key.len());
        let exc = naml_env_error_new(message_ptr, key_ptr);

        let stack = naml_stack_capture();
        *(exc as *mut u8).add(8).cast::<*mut u8>() = stack;

        naml_exception_set_typed(exc as *mut u8, EXCEPTION_TYPE_ENV_ERROR);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_getenv(key: *const NamlString) -> *mut NamlString {
    let key_str = unsafe { string_from_naml(key) };
    let val = std::env::var(&key_str).unwrap_or_default();
    unsafe { naml_from_string(&val) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_lookup_env(key: *const NamlString) -> *mut NamlString {
    let key_str = unsafe { string_from_naml(key) };
    match std::env::var(&key_str) {
        Ok(val) => unsafe { naml_from_string(&val) },
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_setenv(
    key: *const NamlString,
    value: *const NamlString,
) -> i64 {
    let key_str = unsafe { string_from_naml(key) };
    let value_str = unsafe { string_from_naml(value) };

    if key_str.is_empty() || key_str.contains('=') || key_str.contains('\0') {
        throw_env_error(
            &format!("invalid environment variable key: '{}'", key_str),
            &key_str,
        );
        return 0;
    }
    if value_str.contains('\0') {
        throw_env_error("environment variable value contains null byte", &key_str);
        return 0;
    }

    unsafe { std::env::set_var(&key_str, &value_str) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_unsetenv(key: *const NamlString) -> i64 {
    let key_str = unsafe { string_from_naml(key) };

    if key_str.is_empty() || key_str.contains('=') || key_str.contains('\0') {
        throw_env_error(
            &format!("invalid environment variable key: '{}'", key_str),
            &key_str,
        );
        return 0;
    }

    unsafe { std::env::remove_var(&key_str) };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_env_clearenv() -> i64 {
    let keys: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
    for key in keys {
        unsafe { std::env::remove_var(&key) };
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_env_environ() -> *mut NamlArray {
    let vars: Vec<String> = std::env::vars()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let arr = unsafe { naml_array_new(vars.len()) };
    for entry in vars.iter() {
        let s = unsafe { naml_string_new(entry.as_ptr(), entry.len()) };
        unsafe { naml_array_push(arr, s as i64) };
    }
    arr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_env_expand_env(s: *const NamlString) -> *mut NamlString {
    let input = unsafe { string_from_naml(s) };
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' && i + 1 < len {
            if bytes[i + 1] == b'{' {
                if let Some(end) = input[i + 2..].find('}') {
                    let var_name = &input[i + 2..i + 2 + end];
                    let val = std::env::var(var_name).unwrap_or_default();
                    result.push_str(&val);
                    i += 2 + end + 1;
                } else {
                    result.push('$');
                    i += 1;
                }
            } else {
                let start = i + 1;
                let mut end = start;
                while end < len
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > start {
                    let var_name = &input[start..end];
                    let val = std::env::var(var_name).unwrap_or_default();
                    result.push_str(&val);
                    i = end;
                } else {
                    result.push('$');
                    i += 1;
                }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    unsafe { naml_from_string(&result) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_getenv_existing() {
        unsafe {
            std::env::set_var("NAML_TEST_VAR", "hello");
            let key = naml_string_new(b"NAML_TEST_VAR".as_ptr(), 13);
            let result = naml_env_getenv(key);
            let val = string_from_naml(result);
            assert_eq!(val, "hello");
            std::env::remove_var("NAML_TEST_VAR");
        }
    }

    #[test]
    fn test_getenv_missing() {
        unsafe {
            let key = naml_string_new(b"NAML_NONEXISTENT_VAR_XYZ".as_ptr(), 24);
            let result = naml_env_getenv(key);
            let val = string_from_naml(result);
            assert_eq!(val, "");
        }
    }

    #[test]
    fn test_expand_env_dollar() {
        unsafe {
            std::env::set_var("NAML_EXP_TEST", "world");
            let input = naml_string_new(b"hello $NAML_EXP_TEST!".as_ptr(), 21);
            let result = naml_env_expand_env(input);
            let val = string_from_naml(result);
            assert_eq!(val, "hello world!");
            std::env::remove_var("NAML_EXP_TEST");
        }
    }

    #[test]
    fn test_expand_env_braces() {
        unsafe {
            std::env::set_var("NAML_EXP_TEST2", "value");
            let input = naml_string_new(b"test ${NAML_EXP_TEST2} done".as_ptr(), 27);
            let result = naml_env_expand_env(input);
            let val = string_from_naml(result);
            assert_eq!(val, "test value done");
            std::env::remove_var("NAML_EXP_TEST2");
        }
    }
}
