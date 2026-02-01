///
/// std::encoding::utf8 - UTF-8 Encoding/Decoding
///
/// Provides UTF-8 string <-> bytes conversion with validation.
/// - encode(s: string) -> bytes: Convert string to UTF-8 bytes
/// - decode(data: bytes) -> string throws DecodeError: Convert bytes to string with validation
///

use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

/// Encode a string to UTF-8 bytes
/// Since naml strings are already UTF-8 internally, this is essentially a copy
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_utf8_encode(s: *const NamlString) -> *mut NamlBytes {
    if s.is_null() {
        return create_empty_bytes();
    }

    unsafe {
        let len = (*s).len;
        let data = (*s).data.as_ptr();
        create_bytes_from(data, len)
    }
}

/// Decode UTF-8 bytes to a string with validation
/// Returns: pointer to result struct { tag: i32, value: i64 }
/// tag = 0: success, value = string pointer
/// tag = 1: error, value = position of invalid byte
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_utf8_decode(
    b: *const NamlBytes,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if b.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = create_empty_string() as i64;
        }
        return;
    }

    unsafe {
        let len = (*b).len;
        let data = std::slice::from_raw_parts((*b).data.as_ptr(), len);

        match std::str::from_utf8(data) {
            Ok(_valid_str) => {
                let string_ptr = naml_std_core::value::naml_string_new(data.as_ptr(), len);
                *out_tag = 0;
                *out_value = string_ptr as i64;
            }
            Err(e) => {
                *out_tag = 1;
                *out_value = e.valid_up_to() as i64;
            }
        }
    }
}

/// Check if bytes are valid UTF-8
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_utf8_is_valid(b: *const NamlBytes) -> i64 {
    if b.is_null() {
        return 1;
    }

    unsafe {
        let len = (*b).len;
        let data = std::slice::from_raw_parts((*b).data.as_ptr(), len);
        if std::str::from_utf8(data).is_ok() { 1 } else { 0 }
    }
}

fn create_empty_bytes() -> *mut NamlBytes {
    unsafe {
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + 8,
            std::mem::align_of::<NamlBytes>(),
        ).unwrap();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut NamlBytes;
        (*ptr).header = naml_std_core::HeapHeader::new(naml_std_core::HeapTag::Bytes);
        (*ptr).len = 0;
        (*ptr).capacity = 8;
        ptr
    }
}

fn create_bytes_from(data: *const u8, len: usize) -> *mut NamlBytes {
    unsafe {
        let cap = if len == 0 { 8 } else { len };
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        ).unwrap();
        let ptr = std::alloc::alloc(layout) as *mut NamlBytes;
        (*ptr).header = naml_std_core::HeapHeader::new(naml_std_core::HeapTag::Bytes);
        (*ptr).len = len;
        (*ptr).capacity = cap;
        if len > 0 && !data.is_null() {
            std::ptr::copy_nonoverlapping(data, (*ptr).data.as_mut_ptr(), len);
        }
        ptr
    }
}

fn create_empty_string() -> *mut NamlString {
    unsafe { naml_std_core::value::naml_string_new(std::ptr::null(), 0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_encode() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"Hello".as_ptr(), 5);
            let b = naml_encoding_utf8_encode(s);
            assert_eq!((*b).len, 5);
        }
    }

    #[test]
    fn test_utf8_decode_valid() {
        unsafe {
            let bytes = create_bytes_from(b"Hello".as_ptr(), 5);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_utf8_decode(bytes, &mut tag, &mut value);
            assert_eq!(tag, 0);
            assert!(value != 0);
        }
    }

    #[test]
    fn test_utf8_decode_invalid() {
        unsafe {
            let invalid = [0xFF, 0xFE];
            let bytes = create_bytes_from(invalid.as_ptr(), 2);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_utf8_decode(bytes, &mut tag, &mut value);
            assert_eq!(tag, 1);
            assert_eq!(value, 0);
        }
    }
}
