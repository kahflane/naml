///
/// std::crypto - Secure Random Bytes
///
/// Provides cryptographically secure random byte generation using OS entropy
/// via the `rand` crate's OsRng.
///
/// `naml_crypto_random_bytes(n) -> bytes` — Generate n cryptographically secure random bytes
///

use naml_std_core::bytes::NamlBytes;
use std::alloc::Layout;

use rand::RngCore;

fn create_bytes_with_len(len: usize) -> *mut NamlBytes {
    unsafe {
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
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_crypto_random_bytes(n: i64) -> *mut NamlBytes {
    let len = if n <= 0 { 0usize } else { n as usize };
    let ptr = create_bytes_with_len(len);
    if len > 0 {
        unsafe {
            let buf = std::slice::from_raw_parts_mut((*ptr).data.as_mut_ptr(), len);
            rand::rngs::OsRng.fill_bytes(buf);
        }
    }
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_bytes_length() {
        unsafe {
            let result = naml_crypto_random_bytes(32);
            assert_eq!((*result).len, 32);
        }
    }

    #[test]
    fn test_random_bytes_zero() {
        unsafe {
            let result = naml_crypto_random_bytes(0);
            assert_eq!((*result).len, 0);
        }
    }

    #[test]
    fn test_random_bytes_unique() {
        unsafe {
            let r1 = naml_crypto_random_bytes(32);
            let r2 = naml_crypto_random_bytes(32);
            let s1 = std::slice::from_raw_parts((*r1).data.as_ptr(), (*r1).len);
            let s2 = std::slice::from_raw_parts((*r2).data.as_ptr(), (*r2).len);
            assert_ne!(s1, s2);
        }
    }
}
