///
/// Bytes Runtime Support
///
/// Provides heap-allocated byte arrays with reference counting.
/// Similar to strings but for raw binary data.
///
use std::alloc::{alloc, dealloc, Layout};
use super::value::{HeapHeader, HeapTag, NamlString, naml_string_new};

/// A heap-allocated byte array
#[repr(C)]
pub struct NamlBytes {
    pub header: HeapHeader,
    pub len: usize,
    pub capacity: usize,
    pub data: [u8; 0],
}

/// Allocate new bytes with given capacity
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_new(capacity: usize) -> *mut NamlBytes {
    unsafe {
        let cap = if capacity == 0 { 8 } else { capacity };
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        ).unwrap();

        let ptr = alloc(layout) as *mut NamlBytes;
        if ptr.is_null() {
            panic!("Failed to allocate bytes");
        }

        (*ptr).header = HeapHeader::new(HeapTag::String); // Reuse tag for simplicity
        (*ptr).len = 0;
        (*ptr).capacity = cap;

        ptr
    }
}

/// Create bytes from raw data
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_from(data: *const u8, len: usize) -> *mut NamlBytes {
    unsafe {
        let cap = if len == 0 { 8 } else { len };
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlBytes>() + cap,
            std::mem::align_of::<NamlBytes>(),
        ).unwrap();

        let ptr = alloc(layout) as *mut NamlBytes;
        if ptr.is_null() {
            panic!("Failed to allocate bytes");
        }

        (*ptr).header = HeapHeader::new(HeapTag::String);
        (*ptr).len = len;
        (*ptr).capacity = cap;

        if !data.is_null() && len > 0 {
            std::ptr::copy_nonoverlapping(data, (*ptr).data.as_mut_ptr(), len);
        }

        ptr
    }
}

/// Get bytes length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_len(b: *const NamlBytes) -> i64 {
    if b.is_null() {
        0
    } else {
        unsafe { (*b).len as i64 }
    }
}

/// Get byte at index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_get(b: *const NamlBytes, index: i64) -> i64 {
    if b.is_null() {
        return 0;
    }
    unsafe {
        if index < 0 || index as usize >= (*b).len {
            return 0;
        }
        *(*b).data.as_ptr().add(index as usize) as i64
    }
}

/// Set byte at index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_set(b: *mut NamlBytes, index: i64, value: i64) {
    if b.is_null() {
        return;
    }
    unsafe {
        if index >= 0 && (index as usize) < (*b).len {
            *(*b).data.as_mut_ptr().add(index as usize) = value as u8;
        }
    }
}

/// Increment reference count
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_incref(b: *mut NamlBytes) {
    if !b.is_null() {
        unsafe { (*b).header.incref(); }
    }
}

/// Decrement reference count and free if zero
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_decref(b: *mut NamlBytes) {
    if !b.is_null() {
        unsafe {
            if (*b).header.decref() {
                let cap = (*b).capacity;
                let layout = Layout::from_size_align(
                    std::mem::size_of::<NamlBytes>() + cap,
                    std::mem::align_of::<NamlBytes>(),
                ).unwrap();
                dealloc(b as *mut u8, layout);
            }
        }
    }
}

/// Convert bytes to string (UTF-8)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_to_string(b: *const NamlBytes) -> *mut NamlString {
    if b.is_null() {
        return unsafe { naml_string_new(std::ptr::null(), 0) };
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len);
        naml_string_new(slice.as_ptr(), slice.len())
    }
}

/// Convert string to bytes
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_to_bytes(s: *const NamlString) -> *mut NamlBytes {
    if s.is_null() {
        return unsafe { naml_bytes_new(0) };
    }
    unsafe {
        let len = (*s).len;
        naml_bytes_from((*s).data.as_ptr(), len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_creation() {
        unsafe {
            let b = naml_bytes_new(16);
            assert!(!b.is_null());
            assert_eq!((*b).len, 0);
            assert_eq!((*b).capacity, 16);
            naml_bytes_decref(b);
        }
    }

    #[test]
    fn test_bytes_from() {
        unsafe {
            let data = b"hello";
            let b = naml_bytes_from(data.as_ptr(), data.len());
            assert_eq!((*b).len, 5);
            assert_eq!(naml_bytes_get(b, 0), 'h' as i64);
            assert_eq!(naml_bytes_get(b, 4), 'o' as i64);
            naml_bytes_decref(b);
        }
    }
}
