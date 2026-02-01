///
/// std::encoding::url - URL Percent-Encoding/Decoding
///
/// Provides URL encoding (percent-encoding) for strings using the `urlencoding` crate.
/// - encode(s: string) -> string: URL-encode a string
/// - decode(s: string) -> string throws DecodeError: URL-decode a string
///

use naml_std_core::value::NamlString;

/// URL-encode a string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_url_encode(s: *const NamlString) -> *mut NamlString {
    if s.is_null() {
        return unsafe { naml_std_core::value::naml_string_new(std::ptr::null(), 0) };
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        let str_val = std::str::from_utf8_unchecked(data);
        let encoded = urlencoding::encode(str_val);
        naml_std_core::value::naml_string_new(encoded.as_ptr(), encoded.len())
    }
}

/// URL-decode a string
/// Returns via out parameters:
/// tag = 0: success, value = string pointer
/// tag = 1: error, value = position of invalid sequence
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_url_decode(
    s: *const NamlString,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if s.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = naml_std_core::value::naml_string_new(std::ptr::null(), 0) as i64;
        }
        return;
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        let str_val = std::str::from_utf8_unchecked(data);

        // Validate percent sequences first (urlencoding crate passes through invalid ones)
        if let Some(pos) = find_invalid_percent_position(data) {
            *out_tag = 1;
            *out_value = pos as i64;
            return;
        }

        match urlencoding::decode(str_val) {
            Ok(decoded) => {
                let result = naml_std_core::value::naml_string_new(decoded.as_ptr(), decoded.len());
                *out_tag = 0;
                *out_value = result as i64;
            }
            Err(_) => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Find the position of an invalid percent sequence for error reporting
fn find_invalid_percent_position(data: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'%' {
            if i + 2 >= data.len() {
                return Some(i);
            }
            if !is_hex_char(data[i + 1]) || !is_hex_char(data[i + 2]) {
                return Some(i);
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    None
}

fn is_hex_char(c: u8) -> bool {
    matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"hello world".as_ptr(), 11);
            let result = naml_encoding_url_encode(s);
            let encoded = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(encoded, b"hello%20world");
        }
    }

    #[test]
    fn test_url_encode_special() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"a&b=c".as_ptr(), 5);
            let result = naml_encoding_url_encode(s);
            let encoded = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(encoded, b"a%26b%3Dc");
        }
    }

    #[test]
    fn test_url_decode_valid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"hello%20world".as_ptr(), 13);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_url_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            let result = value as *const NamlString;
            let decoded = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(decoded, b"hello world");
        }
    }

    #[test]
    fn test_url_decode_plus() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"hello+world".as_ptr(), 11);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_url_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            let result = value as *const NamlString;
            let decoded = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            // urlencoding crate keeps + as +, doesn't convert to space
            // This is RFC 3986 compliant (+ is only space in application/x-www-form-urlencoded)
            assert_eq!(decoded, b"hello+world");
        }
    }

    #[test]
    fn test_url_decode_invalid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"%ZZ".as_ptr(), 3);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_url_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
            assert_eq!(value, 0);
        }
    }
}
