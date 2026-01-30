//!
//! Runtime Array Type
//!
//! Provides heap-allocated, reference-counted arrays for naml.
//! Arrays are generic over element type at the naml level, but at runtime
//! we store elements as 64-bit values (either primitives or pointers).
//!

use std::alloc::{alloc, dealloc, realloc, Layout};
use crate::value::{HeapHeader, HeapTag, NamlString, naml_string_decref};

/// A heap-allocated array of i64 values
/// (All naml values are represented as i64 at runtime)
#[repr(C)]
pub struct NamlArray {
    pub header: HeapHeader,
    pub len: usize,
    pub capacity: usize,
    pub data: *mut i64,
}

/// Create a new empty array with given initial capacity
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_new(capacity: usize) -> *mut NamlArray {
    unsafe {
        let layout = Layout::new::<NamlArray>();
        let ptr = alloc(layout) as *mut NamlArray;
        if ptr.is_null() {
            panic!("Failed to allocate array");
        }

        let cap = if capacity == 0 { 4 } else { capacity };
        let data_layout = Layout::array::<i64>(cap).unwrap();
        let data = alloc(data_layout) as *mut i64;
        if data.is_null() {
            dealloc(ptr as *mut u8, layout);
            panic!("Failed to allocate array data");
        }

        (*ptr).header = HeapHeader::new(HeapTag::Array);
        (*ptr).len = 0;
        (*ptr).capacity = cap;
        (*ptr).data = data;

        ptr
    }
}

/// Create an array from existing values
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_from(values: *const i64, len: usize) -> *mut NamlArray {
    unsafe {
        let arr = naml_array_new(len);
        if len > 0 && !values.is_null() {
            std::ptr::copy_nonoverlapping(values, (*arr).data, len);
            (*arr).len = len;
        }
        arr
    }
}

/// Increment reference count
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_incref(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe { (*arr).header.incref(); }
    }
}

/// Decrement reference count and free if zero (for arrays of primitives)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                let data_layout = Layout::array::<i64>((*arr).capacity).unwrap();
                dealloc((*arr).data as *mut u8, data_layout);

                let layout = Layout::new::<NamlArray>();
                dealloc(arr as *mut u8, layout);
            }
        }
    }
}

/// Decrement reference count and free if zero, also decref string elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_strings(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        naml_string_decref(elem as *mut NamlString);
                    }
                }

                let data_layout = Layout::array::<i64>((*arr).capacity).unwrap();
                dealloc((*arr).data as *mut u8, data_layout);

                let layout = Layout::new::<NamlArray>();
                dealloc(arr as *mut u8, layout);
            }
        }
    }
}

/// Decrement reference count and free if zero, also decref nested array elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_arrays(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        naml_array_decref(elem as *mut NamlArray);
                    }
                }

                let data_layout = Layout::array::<i64>((*arr).capacity).unwrap();
                dealloc((*arr).data as *mut u8, data_layout);

                let layout = Layout::new::<NamlArray>();
                dealloc(arr as *mut u8, layout);
            }
        }
    }
}

/// Get array length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_len(arr: *const NamlArray) -> i64 {
    if arr.is_null() {
        0
    } else {
        unsafe { (*arr).len as i64 }
    }
}

/// Get element at index (returns 0 if out of bounds)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_get(arr: *const NamlArray, index: i64) -> i64 {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        let idx = index as usize;
        if idx >= (*arr).len {
            return 0;
        }
        *(*arr).data.add(idx)
    }
}

/// Set element at index (no-op if out of bounds)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_set(arr: *mut NamlArray, index: i64, value: i64) {
    if arr.is_null() {
        return;
    }

    unsafe {
        let idx = index as usize;
        if idx < (*arr).len {
            *(*arr).data.add(idx) = value;
        }
    }
}

/// Push element to end of array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_push(arr: *mut NamlArray, value: i64) {
    if arr.is_null() {
        return;
    }

    unsafe {
        if (*arr).len >= (*arr).capacity {
            let new_capacity = (*arr).capacity * 2;
            let old_layout = Layout::array::<i64>((*arr).capacity).unwrap();
            let new_layout = Layout::array::<i64>(new_capacity).unwrap();

            let new_data = realloc((*arr).data as *mut u8, old_layout, new_layout.size()) as *mut i64;
            if new_data.is_null() {
                panic!("Failed to grow array");
            }

            (*arr).data = new_data;
            (*arr).capacity = new_capacity;
        }

        *(*arr).data.add((*arr).len) = value;
        (*arr).len += 1;
    }
}

/// Pop element from end of array (returns 0 if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_pop(arr: *mut NamlArray) -> i64 {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        if (*arr).len == 0 {
            return 0;
        }

        (*arr).len -= 1;
        *(*arr).data.add((*arr).len)
    }
}

/// Check if array contains a value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_contains(arr: *const NamlArray, value: i64) -> i64 {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        for i in 0..(*arr).len {
            if *(*arr).data.add(i) == value {
                return 1;
            }
        }
        0
    }
}

/// Create a copy of the array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_clone(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() {
        return unsafe { naml_array_new(0) };
    }

    unsafe {
        naml_array_from((*arr).data, (*arr).len)
    }
}

/// Check if array is empty
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_is_empty(arr: *const NamlArray) -> i64 {
    if arr.is_null() {
        return 1;
    }
    unsafe {
        if (*arr).len == 0 { 1 } else { 0 }
    }
}

/// Remove and return first element (shift)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_shift(arr: *mut NamlArray) -> i64 {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        if (*arr).len == 0 {
            return 0;
        }

        let first = *(*arr).data;

        if (*arr).len > 1 {
            std::ptr::copy(
                (*arr).data.add(1),
                (*arr).data,
                (*arr).len - 1
            );
        }

        (*arr).len -= 1;
        first
    }
}

/// Fill all elements with a value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_fill(arr: *mut NamlArray, value: i64) {
    if arr.is_null() {
        return;
    }

    unsafe {
        for i in 0..(*arr).len {
            *(*arr).data.add(i) = value;
        }
    }
}

/// Clear the array (set length to 0)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_clear(arr: *mut NamlArray) {
    if arr.is_null() {
        return;
    }
    unsafe {
        (*arr).len = 0;
    }
}

/// Print array contents (for int arrays)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_print(arr: *const NamlArray) {
    if arr.is_null() {
        print!("[]");
        return;
    }

    unsafe {
        print!("[");
        for i in 0..(*arr).len {
            if i > 0 {
                print!(", ");
            }
            print!("{}", *(*arr).data.add(i));
        }
        print!("]");
    }
}

/// Print array contents (for string arrays)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_print_strings(arr: *const NamlArray) {
    if arr.is_null() {
        print!("[]");
        return;
    }

    unsafe {
        print!("[");
        for i in 0..(*arr).len {
            if i > 0 {
                print!(", ");
            }
            let str_ptr = *(*arr).data.add(i) as *const NamlString;
            if !str_ptr.is_null() {
                print!("\"{}\"", (*str_ptr).as_str());
            } else {
                print!("null");
            }
        }
        print!("]");
    }
}

/// Decrement reference count and free if zero, also decref map elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_maps(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                let data_layout = Layout::array::<i64>((*arr).capacity).unwrap();
                dealloc((*arr).data as *mut u8, data_layout);

                let layout = Layout::new::<NamlArray>();
                dealloc(arr as *mut u8, layout);
            }
        }
    }
}

/// Decrement reference count and free if zero, also decref struct elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_structs(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        crate::value::naml_struct_decref(elem as *mut crate::value::NamlStruct);
                    }
                }

                let data_layout = Layout::array::<i64>((*arr).capacity).unwrap();
                dealloc((*arr).data as *mut u8, data_layout);

                let layout = Layout::new::<NamlArray>();
                dealloc(arr as *mut u8, layout);
            }
        }
    }
}

/// Reverse array in place and return it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_reverse(arr: *mut NamlArray) -> *mut NamlArray {
    unsafe {
        if arr.is_null() || (*arr).len <= 1 {
            return arr;
        }
        let len = (*arr).len;
        for i in 0..len / 2 {
            let j = len - 1 - i;
            let tmp = *(*arr).data.add(i);
            *(*arr).data.add(i) = *(*arr).data.add(j);
            *(*arr).data.add(j) = tmp;
        }
        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_creation() {
        unsafe {
            let arr = naml_array_new(10);
            assert!(!arr.is_null());
            assert_eq!((*arr).len, 0);
            assert_eq!((*arr).capacity, 10);
            naml_array_decref(arr);
        }
    }

    #[test]
    fn test_array_push_get() {
        unsafe {
            let arr = naml_array_new(2);
            naml_array_push(arr, 10);
            naml_array_push(arr, 20);
            naml_array_push(arr, 30);

            assert_eq!(naml_array_len(arr), 3);
            assert_eq!(naml_array_get(arr, 0), 10);
            assert_eq!(naml_array_get(arr, 1), 20);
            assert_eq!(naml_array_get(arr, 2), 30);

            naml_array_decref(arr);
        }
    }
}
