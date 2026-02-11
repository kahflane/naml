///
/// Bytes Runtime Support
///
/// Provides heap-allocated byte arrays with reference counting.
/// Similar to strings but for raw binary data.
///
use std::alloc::{alloc, dealloc, Layout};
use naml_std_core::{HeapHeader, HeapTag, NamlString, naml_string_new};

pub use naml_std_core::NamlBytes;

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

        (*ptr).header = HeapHeader::new(HeapTag::String);
        (*ptr).len = 0;
        (*ptr).capacity = cap;

        ptr
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_len(b: *const NamlBytes) -> i64 {
    if b.is_null() {
        0
    } else {
        unsafe { (*b).len as i64 }
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_bytes_incref(b: *mut NamlBytes) {
    if !b.is_null() {
        unsafe { (*b).header.incref(); }
    }
}

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
