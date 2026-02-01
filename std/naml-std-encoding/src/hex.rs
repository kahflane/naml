///
/// std::encoding::hex - Hexadecimal Encoding/Decoding
///
/// Provides bytes <-> hex string conversion using the `hex` crate.
/// - encode(data: bytes) -> string: Convert bytes to lowercase hex string
/// - decode(s: string) -> bytes throws DecodeError: Convert hex string to bytes
///

use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

/// Encode bytes to hexadecimal string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_hex_encode(b: *const NamlBytes) -> *mut NamlString {
    if b.is_null() {
        return unsafe { naml_std_core::value::naml_string_new(std::ptr::null(), 0) };
    }

    unsafe {
        let len = (*b).len;
        let data = std::slice::from_raw_parts((*b).data.as_ptr(), len);
        let hex_string = hex::encode(data);
        naml_std_core::value::naml_string_new(hex_string.as_ptr(), hex_string.len())
    }
}

/// Decode hexadecimal string to bytes
/// Returns via out parameters:
/// tag = 0: success, value = bytes pointer
/// tag = 1: error, value = position of invalid character
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_hex_decode(
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
        let hex_str = std::str::from_utf8_unchecked(data);

        match hex::decode(hex_str) {
            Ok(bytes) => {
                let result = create_bytes_from(bytes.as_ptr(), bytes.len());
                *out_tag = 0;
                *out_value = result as i64;
            }
            Err(e) => {
                *out_tag = 1;
                *out_value = match e {
                    hex::FromHexError::InvalidHexCharacter { index, .. } => index as i64,
                    hex::FromHexError::OddLength => (len - 1) as i64,
                    _ => 0,
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
    fn test_hex_encode() {
        unsafe {
            let data = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
            let bytes = create_bytes_from(data.as_ptr(), 5);
            let result = naml_encoding_hex_encode(bytes);
            let s = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(s, b"48656c6c6f");
        }
    }

    #[test]
    fn test_hex_decode_valid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"48656c6c6f".as_ptr(), 10);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_hex_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            let bytes = value as *const NamlBytes;
            assert_eq!((*bytes).len, 5);
        }
    }

    #[test]
    fn test_hex_decode_invalid() {
        unsafe {
            let s = naml_std_core::value::naml_string_new(b"48ZZ".as_ptr(), 4);
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_hex_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
            assert_eq!(value, 2);
        }
    }
}
