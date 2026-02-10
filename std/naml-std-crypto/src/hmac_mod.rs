///
/// std::crypto - HMAC Functions
///
/// Provides HMAC-SHA256 and HMAC-SHA512 message authentication codes
/// with constant-time verification using the `hmac` crate.
///
/// Functions:
/// - `naml_crypto_hmac_sha256(key, data) -> bytes` — HMAC-SHA256 tag
/// - `naml_crypto_hmac_sha256_hex(key, data) -> string` — HMAC-SHA256 hex
/// - `naml_crypto_hmac_sha512(key, data) -> bytes` — HMAC-SHA512 tag
/// - `naml_crypto_hmac_sha512_hex(key, data) -> string` — HMAC-SHA512 hex
/// - `naml_crypto_hmac_verify_sha256(key, data, mac) -> bool` — constant-time verify
/// - `naml_crypto_hmac_verify_sha512(key, data, mac) -> bool` — constant-time verify
///

use naml_std_core::bytes::NamlBytes;
use naml_std_core::value::NamlString;
use std::alloc::Layout;

use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};

type HmacSha256 = Hmac<Sha256>;
type HmacSha512 = Hmac<Sha512>;

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
pub unsafe extern "C" fn naml_crypto_hmac_sha256(
    key: *const NamlBytes,
    data: *const NamlBytes,
) -> *mut NamlBytes {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mut mac = HmacSha256::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    let result = mac.finalize().into_bytes();
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_hmac_sha256_hex(
    key: *const NamlBytes,
    data: *const NamlBytes,
) -> *mut NamlString {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mut mac = HmacSha256::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    let result = mac.finalize().into_bytes();
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_hmac_sha512(
    key: *const NamlBytes,
    data: *const NamlBytes,
) -> *mut NamlBytes {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mut mac = HmacSha512::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    let result = mac.finalize().into_bytes();
    create_bytes_from(result.as_ref())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_hmac_sha512_hex(
    key: *const NamlBytes,
    data: *const NamlBytes,
) -> *mut NamlString {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mut mac = HmacSha512::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    let result = mac.finalize().into_bytes();
    let hex_str = hex::encode(result);
    create_string_from(&hex_str)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_hmac_verify_sha256(
    key: *const NamlBytes,
    data: *const NamlBytes,
    mac_bytes: *const NamlBytes,
) -> i64 {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mac_slice = bytes_as_slice(mac_bytes);
    let mut mac = HmacSha256::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    if mac.verify_slice(mac_slice).is_ok() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_hmac_verify_sha512(
    key: *const NamlBytes,
    data: *const NamlBytes,
    mac_bytes: *const NamlBytes,
) -> i64 {
    let key_slice = bytes_as_slice(key);
    let data_slice = bytes_as_slice(data);
    let mac_slice = bytes_as_slice(mac_bytes);
    let mut mac = HmacSha512::new_from_slice(key_slice).expect("HMAC accepts any key length");
    mac.update(data_slice);
    if mac.verify_slice(mac_slice).is_ok() { 1 } else { 0 }
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
    fn test_hmac_sha256_known_vector() {
        unsafe {
            let key = make_bytes(b"key");
            let data = make_bytes(b"The quick brown fox jumps over the lazy dog");
            let hex = naml_crypto_hmac_sha256_hex(key, data);
            assert_eq!(
                read_hex_string(hex),
                "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
            );
        }
    }

    #[test]
    fn test_hmac_verify_sha256_valid() {
        unsafe {
            let key = make_bytes(b"secret");
            let data = make_bytes(b"message");
            let mac = naml_crypto_hmac_sha256(key, data);
            let result = naml_crypto_hmac_verify_sha256(key, data, mac);
            assert_eq!(result, 1);
        }
    }

    #[test]
    fn test_hmac_verify_sha256_invalid() {
        unsafe {
            let key = make_bytes(b"secret");
            let data = make_bytes(b"message");
            let bad_mac = make_bytes(b"not a valid mac at all!!!!!!!!!!!");
            let result = naml_crypto_hmac_verify_sha256(key, data, bad_mac);
            assert_eq!(result, 0);
        }
    }

    #[test]
    fn test_hmac_sha512_length() {
        unsafe {
            let key = make_bytes(b"key");
            let data = make_bytes(b"data");
            let mac = naml_crypto_hmac_sha512(key, data);
            assert_eq!((*mac).len, 64);
        }
    }

    #[test]
    fn test_hmac_verify_sha512_roundtrip() {
        unsafe {
            let key = make_bytes(b"my-secret-key");
            let data = make_bytes(b"important payload");
            let mac = naml_crypto_hmac_sha512(key, data);
            assert_eq!(naml_crypto_hmac_verify_sha512(key, data, mac), 1);

            let wrong_data = make_bytes(b"tampered payload");
            assert_eq!(naml_crypto_hmac_verify_sha512(key, wrong_data, mac), 0);
        }
    }
}
