///
/// std::crypto - Key Derivation Functions
///
/// Provides PBKDF2-SHA-256 key derivation using the `pbkdf2` crate.
/// Used for password hashing, key stretching, and deriving encryption keys.
///
/// `naml_crypto_pbkdf2_sha256(password, salt, iterations, key_len) -> bytes`
///
/// The output is deterministic: same inputs always produce the same derived key.
///

use naml_std_core::bytes::NamlBytes;
use std::alloc::Layout;

use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

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

fn bytes_as_slice(b: *const NamlBytes) -> &'static [u8] {
    unsafe {
        if b.is_null() {
            return &[];
        }
        std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_pbkdf2_sha256(
    password: *const NamlBytes,
    salt: *const NamlBytes,
    iterations: i64,
    key_len: i64,
) -> *mut NamlBytes {
    let password_slice = bytes_as_slice(password);
    let salt_slice = bytes_as_slice(salt);
    let iter_count = if iterations <= 0 { 1u32 } else { iterations as u32 };
    let out_len = if key_len <= 0 { 32usize } else { key_len as usize };

    let mut derived = vec![0u8; out_len];
    pbkdf2_hmac::<Sha256>(password_slice, salt_slice, iter_count, &mut derived);
    create_bytes_from(&derived)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bytes(data: &[u8]) -> *mut NamlBytes {
        create_bytes_from(data)
    }

    #[test]
    fn test_pbkdf2_deterministic() {
        unsafe {
            let password = make_bytes(b"password");
            let salt = make_bytes(b"salt");
            let result1 = naml_crypto_pbkdf2_sha256(password, salt, 1000, 32);
            let result2 = naml_crypto_pbkdf2_sha256(password, salt, 1000, 32);
            let s1 = std::slice::from_raw_parts((*result1).data.as_ptr(), (*result1).len);
            let s2 = std::slice::from_raw_parts((*result2).data.as_ptr(), (*result2).len);
            assert_eq!(s1, s2);
        }
    }

    #[test]
    fn test_pbkdf2_output_length() {
        unsafe {
            let password = make_bytes(b"pw");
            let salt = make_bytes(b"s");
            let result = naml_crypto_pbkdf2_sha256(password, salt, 1, 64);
            assert_eq!((*result).len, 64);
        }
    }

    #[test]
    fn test_pbkdf2_different_salts() {
        unsafe {
            let password = make_bytes(b"password");
            let salt1 = make_bytes(b"salt1");
            let salt2 = make_bytes(b"salt2");
            let r1 = naml_crypto_pbkdf2_sha256(password, salt1, 100, 32);
            let r2 = naml_crypto_pbkdf2_sha256(password, salt2, 100, 32);
            let s1 = std::slice::from_raw_parts((*r1).data.as_ptr(), (*r1).len);
            let s2 = std::slice::from_raw_parts((*r2).data.as_ptr(), (*r2).len);
            assert_ne!(s1, s2);
        }
    }

    #[test]
    fn test_pbkdf2_different_iterations() {
        unsafe {
            let password = make_bytes(b"password");
            let salt = make_bytes(b"salt");
            let r1 = naml_crypto_pbkdf2_sha256(password, salt, 1, 32);
            let r2 = naml_crypto_pbkdf2_sha256(password, salt, 2, 32);
            let s1 = std::slice::from_raw_parts((*r1).data.as_ptr(), (*r1).len);
            let s2 = std::slice::from_raw_parts((*r2).data.as_ptr(), (*r2).len);
            assert_ne!(s1, s2);
        }
    }
}
