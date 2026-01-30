#![allow(unsafe_op_in_unsafe_fn)]
//!
//! naml-std-strings - String Manipulation Functions
//!
//! Provides string helper functions for naml programs:
//!
//! ## Case Conversion
//! - `upper(s: string) -> string` - Convert to uppercase
//! - `lower(s: string) -> string` - Convert to lowercase
//!
//! ## Search Functions
//! - `has(s: string, substr: string) -> bool` - Check if contains substring
//! - `starts_with(s: string, prefix: string) -> bool` - Check prefix
//! - `ends_with(s: string, suffix: string) -> bool` - Check suffix
//!
//! ## Replacement
//! - `replace(s: string, old: string, new: string) -> string` - Replace first occurrence
//! - `replace_all(s: string, old: string, new: string) -> string` - Replace all
//!
//! ## Trimming
//! - `ltrim(s: string) -> string` - Trim whitespace from start
//! - `rtrim(s: string) -> string` - Trim whitespace from end
//!
//! ## Extraction
//! - `substr(s: string, start: int, end: int) -> string` - Get substring
//!
//! ## Padding
//! - `lpad(s: string, len: int, char: string) -> string` - Pad from start
//! - `rpad(s: string, len: int, char: string) -> string` - Pad from end
//!
//! ## Other
//! - `repeat(s: string, n: int) -> string` - Repeat n times
//!
//! ## Splitting (returns arrays)
//! - `split(s: string, delim: string) -> [string]` - Split by delimiter
//! - `lines(s: string) -> [string]` - Split by newlines
//! - `chars(s: string) -> [string]` - Split into characters
//!
//! ## Joining
//! - `concat(arr: [string], delim: string) -> string` - Join array with delimiter
//!

use naml_std_core::{NamlString, NamlArray, naml_string_new, naml_string_incref, naml_array_new, naml_array_push};

/// Convert string to uppercase
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_upper(s: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let upper = str_val.to_uppercase();
        naml_string_new(upper.as_ptr(), upper.len())
    }
}

/// Convert string to lowercase
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_lower(s: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let lower = str_val.to_lowercase();
        naml_string_new(lower.as_ptr(), lower.len())
    }
}

/// Check if string contains a substring (has)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_contains(s: *const NamlString, substr: *const NamlString) -> i64 {
    unsafe {
        if s.is_null() || substr.is_null() {
            return 0;
        }
        let str_val = (*s).as_str();
        let sub_val = (*substr).as_str();
        if str_val.contains(sub_val) { 1 } else { 0 }
    }
}

/// Check if string starts with a prefix
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_starts_with(s: *const NamlString, prefix: *const NamlString) -> i64 {
    unsafe {
        if s.is_null() || prefix.is_null() {
            return 0;
        }
        let str_val = (*s).as_str();
        let prefix_val = (*prefix).as_str();
        if str_val.starts_with(prefix_val) { 1 } else { 0 }
    }
}

/// Check if string ends with a suffix
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_ends_with(s: *const NamlString, suffix: *const NamlString) -> i64 {
    unsafe {
        if s.is_null() || suffix.is_null() {
            return 0;
        }
        let str_val = (*s).as_str();
        let suffix_val = (*suffix).as_str();
        if str_val.ends_with(suffix_val) { 1 } else { 0 }
    }
}

/// Replace first occurrence of old with new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_replace(s: *const NamlString, old: *const NamlString, new: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        if old.is_null() || new.is_null() {
            naml_string_incref(s as *mut NamlString);
            return s as *mut NamlString;
        }
        let str_val = (*s).as_str();
        let old_val = (*old).as_str();
        let new_val = (*new).as_str();
        let result = str_val.replacen(old_val, new_val, 1);
        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Replace all occurrences of old with new
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_replace_all(s: *const NamlString, old: *const NamlString, new: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        if old.is_null() || new.is_null() {
            naml_string_incref(s as *mut NamlString);
            return s as *mut NamlString;
        }
        let str_val = (*s).as_str();
        let old_val = (*old).as_str();
        let new_val = (*new).as_str();
        let result = str_val.replace(old_val, new_val);
        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Trim whitespace from start of string (ltrim)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_ltrim(s: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let trimmed = str_val.trim_start();
        naml_string_new(trimmed.as_ptr(), trimmed.len())
    }
}

/// Trim whitespace from end of string (rtrim)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_rtrim(s: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let trimmed = str_val.trim_end();
        naml_string_new(trimmed.as_ptr(), trimmed.len())
    }
}

/// Get substring from start to end (exclusive), using character indices
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_substr(s: *const NamlString, start: i64, end: i64) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let char_indices: Vec<(usize, char)> = str_val.char_indices().collect();
        let len = char_indices.len();

        let start_idx = std::cmp::max(0, start) as usize;
        let end_idx = std::cmp::min(end as usize, len);

        if start_idx >= end_idx || start_idx >= len {
            return naml_string_new(std::ptr::null(), 0);
        }

        let byte_start = char_indices[start_idx].0;
        let byte_end = if end_idx >= len {
            str_val.len()
        } else {
            char_indices[end_idx].0
        };

        let substr = &str_val[byte_start..byte_end];
        naml_string_new(substr.as_ptr(), substr.len())
    }
}

/// Pad string from the start with a character to reach target length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_lpad(s: *const NamlString, target_len: i64, pad_char: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let char_count = str_val.chars().count();
        let target = target_len as usize;

        if char_count >= target {
            return naml_string_new(str_val.as_ptr(), str_val.len());
        }

        let pad = if pad_char.is_null() || (*pad_char).len == 0 {
            ' '
        } else {
            (*pad_char).as_str().chars().next().unwrap_or(' ')
        };

        let pad_count = target - char_count;
        let mut result = String::with_capacity(str_val.len() + pad_count * pad.len_utf8());
        for _ in 0..pad_count {
            result.push(pad);
        }
        result.push_str(str_val);
        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Pad string from the end with a character to reach target length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_rpad(s: *const NamlString, target_len: i64, pad_char: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let char_count = str_val.chars().count();
        let target = target_len as usize;

        if char_count >= target {
            return naml_string_new(str_val.as_ptr(), str_val.len());
        }

        let pad = if pad_char.is_null() || (*pad_char).len == 0 {
            ' '
        } else {
            (*pad_char).as_str().chars().next().unwrap_or(' ')
        };

        let pad_count = target - char_count;
        let mut result = String::with_capacity(str_val.len() + pad_count * pad.len_utf8());
        result.push_str(str_val);
        for _ in 0..pad_count {
            result.push(pad);
        }
        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Repeat string n times
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_repeat(s: *const NamlString, n: i64) -> *mut NamlString {
    unsafe {
        if s.is_null() || n <= 0 {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let result = str_val.repeat(n as usize);
        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Split a string by delimiter and return array of strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_split(s: *const NamlString, delim: *const NamlString) -> *mut NamlArray {
    unsafe {
        if s.is_null() {
            return naml_array_new(0);
        }

        let str_val = (*s).as_str();
        let delim_val = if delim.is_null() { "" } else { (*delim).as_str() };

        let parts: Vec<&str> = if delim_val.is_empty() {
            str_val.chars().map(|c| {
                let start = str_val.as_ptr();
                let offset = str_val.char_indices()
                    .find(|(_, ch)| *ch == c)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(start.add(offset), c.len_utf8()))
            }).collect()
        } else {
            str_val.split(delim_val).collect()
        };

        let arr = naml_array_new(parts.len());
        for part in parts {
            let part_str = naml_string_new(part.as_ptr(), part.len());
            naml_array_push(arr, part_str as i64);
        }

        arr
    }
}

/// Join array of strings with delimiter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_join(arr: *const NamlArray, delim: *const NamlString) -> *mut NamlString {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return naml_string_new(std::ptr::null(), 0);
        }

        let delim_val = if delim.is_null() { "" } else { (*delim).as_str() };

        let mut result = String::new();
        for i in 0..(*arr).len {
            if i > 0 {
                result.push_str(delim_val);
            }
            let str_ptr = *(*arr).data.add(i) as *const NamlString;
            if !str_ptr.is_null() {
                result.push_str((*str_ptr).as_str());
            }
        }

        naml_string_new(result.as_ptr(), result.len())
    }
}

/// Split string into lines
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_lines(s: *const NamlString) -> *mut NamlArray {
    unsafe {
        if s.is_null() {
            return naml_array_new(0);
        }
        let str_val = (*s).as_str();
        let lines: Vec<&str> = str_val.lines().collect();
        let arr = naml_array_new(lines.len());
        for line in lines {
            let line_str = naml_string_new(line.as_ptr(), line.len());
            naml_array_push(arr, line_str as i64);
        }
        arr
    }
}

/// Split string into individual characters
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_chars(s: *const NamlString) -> *mut NamlArray {
    unsafe {
        if s.is_null() {
            return naml_array_new(0);
        }
        let str_val = (*s).as_str();
        let chars: Vec<char> = str_val.chars().collect();
        let arr = naml_array_new(chars.len());
        for c in chars {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            let char_str = naml_string_new(s.as_ptr(), s.len());
            naml_array_push(arr, char_str as i64);
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ltrim() {
        unsafe {
            let s = naml_string_new("  hello".as_ptr(), 7);
            let result = naml_string_ltrim(s);
            assert_eq!((*result).as_str(), "hello");
        }
    }

    #[test]
    fn test_rtrim() {
        unsafe {
            let s = naml_string_new("hello  ".as_ptr(), 7);
            let result = naml_string_rtrim(s);
            assert_eq!((*result).as_str(), "hello");
        }
    }

    #[test]
    fn test_substr() {
        unsafe {
            let s = naml_string_new("hello world".as_ptr(), 11);
            let result = naml_string_substr(s, 0, 5);
            assert_eq!((*result).as_str(), "hello");
        }
    }

    #[test]
    fn test_repeat() {
        unsafe {
            let s = naml_string_new("ab".as_ptr(), 2);
            let result = naml_string_repeat(s, 3);
            assert_eq!((*result).as_str(), "ababab");
        }
    }
}
