///
/// std::crypto - Hashing Functions
///
/// Provides MD5, SHA-1, SHA-256, SHA-512 hashing with both raw byte and hex string output.
/// Uses the RustCrypto digest crates (md-5, sha1, sha2).
///
/// Each hash algorithm has two variants:
/// - `naml_crypto_<algo>(data) -> bytes` — raw digest bytes
/// - `naml_crypto_<algo>_hex(data) -> string` — lowercase hex-encoded digest string
///

use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512, Digest};

fn create_bytes_from(data: &[u8]) -> *mut NamlBytes {
    unsafe {
        let len = data.len();
        let cap = if len == 0 { 8 } else { len };
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        ).unwrap();
        let ptr = std::alloc::alloc(layout) as *mut NamlBytes;
        if ptr.is_null() {
            panic!("Failed to allocate bytes");
        }
        (*ptr).header = naml_std_core::HeapHeader::new(naml_std_core::HeapTag::Bytes);
        (*ptr).len = len;
        (*ptr).capacity = cap;
        if len > 0 {
            std::ptr::copy_nonoverlapping(data.as_ptr(), (*ptr).data.as_mut_ptr(), len);
        }
        ptr
    }
}

fn create_string_from(s: &str) -> *mut NamlString {
    unsafe {
        naml_std_core::value::naml_string_new(s.as_ptr(), s.len())
    }
}

fn bytes_as_slice(b: *const NamlBytes) -> &'static [u8] {
    unsafe {
        if b.is_null() {
            return &[];
        }
        std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_md5(data: *const NamlBytes) -> *mut NamlBytes {
    let input = bytes_as_slice(data);
    let result = Md5::digest(input);
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_md5_hex(data: *const NamlBytes) -> *mut NamlString {
    let input = bytes_as_slice(data);
    let result = Md5::digest(input);
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha1(data: *const NamlBytes) -> *mut NamlBytes {
    let input = bytes_as_slice(data);
    let result = Sha1::digest(input);
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha1_hex(data: *const NamlBytes) -> *mut NamlString {
    let input = bytes_as_slice(data);
    let result = Sha1::digest(input);
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha256(data: *const NamlBytes) -> *mut NamlBytes {
    let input = bytes_as_slice(data);
    let result = Sha256::digest(input);
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha256_hex(data: *const NamlBytes) -> *mut NamlString {
    let input = bytes_as_slice(data);
    let result = Sha256::digest(input);
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha512(data: *const NamlBytes) -> *mut NamlBytes {
    let input = bytes_as_slice(data);
    let result = Sha512::digest(input);
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_sha512_hex(data: *const NamlBytes) -> *mut NamlString {
    let input = bytes_as_slice(data);
    let result = Sha512::digest(input);
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bytes(data: &[u8]) -> *mut NamlBytes {
        create_bytes_from(data)
    }

    fn read_hex_string(s: *const NamlString) -> String {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            std::str::from_utf8(slice).unwrap().to_string()
        }
    }

    #[test]
    fn test_md5_known_vector() {
        unsafe {
            let data = make_bytes(b"hello world");
            let hex = naml_crypto_md5_hex(data);
            assert_eq!(read_hex_string(hex), "5eb63bbbe01eeed093cb22bb8f5acdc3");
        }
    }

    #[test]
    fn test_sha1_known_vector() {
        unsafe {
            let data = make_bytes(b"hello world");
            let hex = naml_crypto_sha1_hex(data);
            assert_eq!(read_hex_string(hex), "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
        }
    }

    #[test]
    fn test_sha256_known_vector() {
        unsafe {
            let data = make_bytes(b"hello world");
            let hex = naml_crypto_sha256_hex(data);
            assert_eq!(
                read_hex_string(hex),
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
            );
        }
    }

    #[test]
    fn test_sha512_known_vector() {
        unsafe {
            let data = make_bytes(b"hello world");
            let hex = naml_crypto_sha512_hex(data);
            let expected = "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f";
            assert_eq!(read_hex_string(hex), expected);
        }
    }

    #[test]
    fn test_md5_raw_length() {
        unsafe {
            let data = make_bytes(b"test");
            let result = naml_crypto_md5(data);
            assert_eq!((*result).len, 16);
        }
    }

    #[test]
    fn test_sha256_raw_length() {
        unsafe {
            let data = make_bytes(b"test");
            let result = naml_crypto_sha256(data);
            assert_eq!((*result).len, 32);
        }
    }

    #[test]
    fn test_sha512_raw_length() {
        unsafe {
            let data = make_bytes(b"test");
            let result = naml_crypto_sha512(data);
            assert_eq!((*result).len, 64);
        }
    }

    #[test]
    fn test_empty_input() {
        unsafe {
            let data = make_bytes(b"");
            let hex = naml_crypto_sha256_hex(data);
            assert_eq!(
                read_hex_string(hex),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
        }
    }
}
