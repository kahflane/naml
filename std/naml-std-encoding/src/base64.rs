///
/// std::encoding::base64 - Base64 Encoding/Decoding
///
/// Provides bytes <-> base64 string conversion (RFC 4648) using the `base64` crate.
/// - encode(data: bytes) -> string: Convert bytes to base64 string
/// - decode(s: string) -> bytes throws DecodeError: Convert base64 string to bytes
///

use base64::{Engine, engine::general_purpose::STANDARD};
use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

/// Encode bytes to base64 string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_base64_encode(b: *const NamlBytes) -> *mut NamlString {
    if b.is_null() {
        return unsafe { naml_std_core::value::naml_string_new(std::ptr::null(), 0) };
    }

    unsafe {
        let len = (*b).len;
        let data = std::slice::from_raw_parts((*b).data.as_ptr(), len);
        let b64_string = STANDARD.encode(data);
        naml_std_core::value::naml_string_new(b64_string.as_ptr(), b64_string.len())
    }
}

/// Decode base64 string to bytes
/// Returns via out parameters:
/// tag = 0: success, value = bytes pointer
/// tag = 1: error, value = position of invalid character
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_base64_decode(
    s: *const NamlString,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if s.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = create_empty_bytes() as i64;
        }
        return;
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);

        match STANDARD.decode(data) {
            Ok(bytes) => {
                let result = create_bytes_from(bytes.as_ptr(), bytes.len());
                *out_tag = 0;
                *out_value = result as i64;
            }
            Err(e) => {
                *out_tag = 1;
                *out_value = match e {
                    base64::DecodeError::InvalidByte(pos, _) => pos as i64,
                    base64::DecodeError::InvalidLength(_) => len as i64,
                    base64::DecodeError::InvalidLastSymbol(pos, _) => pos as i64,
                    base64::DecodeError::InvalidPadding => len as i64,
                };
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        unsafe {
            let data = b"Hello";
            let bytes = create_bytes_from(data.as_ptr(), 5);
            let result = naml_encoding_base64_encode(bytes);
            let s = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(s, b"SGVsbG8=");
        }
    }

    #[test]
    fn test_base64_decode_valid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"SGVsbG8=".as_ptr(), 8);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_base64_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            let bytes = value as *const NamlBytes;
            let decoded = std::slice::from_raw_parts((*bytes).data.as_ptr(), (*bytes).len);
            assert_eq!(decoded, b"Hello");
        }
    }

    #[test]
    fn test_base64_decode_invalid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"!!!".as_ptr(), 3);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_base64_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
        }
    }
}
