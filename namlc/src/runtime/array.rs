//!
//! Runtime Array Operations
//!
//! Provides heap-allocated, reference-counted arrays for naml.
//! Arrays are generic over element type at the naml level, but at runtime
//! we store elements as 64-bit values (either primitives or pointers).
//!

use std::alloc::{alloc, dealloc, realloc, Layout};

use naml_std_core::{HeapHeader, HeapTag};

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
                // Decref each string element
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        naml_std_core::naml_string_decref(elem as *mut naml_std_core::NamlString);
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
                // Decref each nested array element
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

/// Decrement reference count and free if zero, also decref map elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_maps(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                // Decref each map element
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        super::map::naml_map_decref(elem as *mut super::map::NamlMap);
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

/// Decrement reference count and free if zero, also decref struct elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_decref_structs(arr: *mut NamlArray) {
    if !arr.is_null() {
        unsafe {
            if (*arr).header.decref() {
                // Decref each struct element
                for i in 0..(*arr).len {
                    let elem = *(*arr).data.add(i);
                    if elem != 0 {
                        naml_std_core::naml_struct_decref(elem as *mut naml_std_core::NamlStruct);
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
            // Grow the array
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
/// Returns the value at index 0 and shifts all elements left
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

        // Shift all elements left by one
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

/// Print array contents (for debugging)
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

/// Split a string by delimiter and return array of strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_split(s: *const naml_std_core::NamlString, delim: *const naml_std_core::NamlString) -> *mut NamlArray {
    unsafe {
        if s.is_null() {
            return naml_array_new(0);
        }

        let str_val = (*s).as_str();
        let delim_val = if delim.is_null() { "" } else { (*delim).as_str() };

        let parts: Vec<&str> = if delim_val.is_empty() {
            str_val.chars().map(|c| {
                let start = str_val.as_ptr();
                let offset = str_val.char_indices()
                    .find(|(_, ch)| *ch == c)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(start.add(offset), c.len_utf8()))
            }).collect()
        } else {
            str_val.split(delim_val).collect()
        };

        let arr = naml_array_new(parts.len());
        for part in parts {
            let part_str = naml_std_core::naml_string_new(part.as_ptr(), part.len());
            naml_array_push(arr, part_str as i64);
        }

        arr
    }
}

/// Join array of strings with delimiter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_join(arr: *const NamlArray, delim: *const naml_std_core::NamlString) -> *mut naml_std_core::NamlString {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return naml_std_core::naml_string_new(std::ptr::null(), 0);
        }

        let delim_val = if delim.is_null() { "" } else { (*delim).as_str() };

        let mut result = String::new();
        for i in 0..(*arr).len {
            if i > 0 {
                result.push_str(delim_val);
            }
            let str_ptr = *(*arr).data.add(i) as *const naml_std_core::NamlString;
            if !str_ptr.is_null() {
                result.push_str((*str_ptr).as_str());
            }
        }

        naml_std_core::naml_string_new(result.as_ptr(), result.len())
    }
}

/// Get first element of array (returns 0 if empty, use with option wrapper)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_first(arr: *const NamlArray) -> i64 {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return 0;
        }
        *(*arr).data
    }
}

/// Get last element of array (returns 0 if empty, use with option wrapper)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_last(arr: *const NamlArray) -> i64 {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return 0;
        }
        *(*arr).data.add((*arr).len - 1)
    }
}

/// Sum all elements (assumes int array)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_sum(arr: *const NamlArray) -> i64 {
    unsafe {
        if arr.is_null() {
            return 0;
        }
        let mut sum: i64 = 0;
        for i in 0..(*arr).len {
            sum = sum.wrapping_add(*(*arr).data.add(i));
        }
        sum
    }
}

/// Find minimum element (returns i64::MAX if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_min(arr: *const NamlArray) -> i64 {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return i64::MAX;
        }
        let mut min = *(*arr).data;
        for i in 1..(*arr).len {
            let val = *(*arr).data.add(i);
            if val < min {
                min = val;
            }
        }
        min
    }
}

/// Find maximum element (returns i64::MIN if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_max(arr: *const NamlArray) -> i64 {
    unsafe {
        if arr.is_null() || (*arr).len == 0 {
            return i64::MIN;
        }
        let mut max = *(*arr).data;
        for i in 1..(*arr).len {
            let val = *(*arr).data.add(i);
            if val > max {
                max = val;
            }
        }
        max
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

/// Create a new reversed copy of array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_reversed(arr: *const NamlArray) -> *mut NamlArray {
    unsafe {
        if arr.is_null() {
            return naml_array_new(0);
        }
        let len = (*arr).len;
        let new_arr = naml_array_new(len);
        for i in 0..len {
            let val = *(*arr).data.add(len - 1 - i);
            naml_array_push(new_arr, val);
        }
        new_arr
    }
}

/// Take first n elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_take(arr: *const NamlArray, n: i64) -> *mut NamlArray {
    unsafe {
        if arr.is_null() || n <= 0 {
            return naml_array_new(0);
        }
        let take_count = std::cmp::min(n as usize, (*arr).len);
        let new_arr = naml_array_new(take_count);
        for i in 0..take_count {
            naml_array_push(new_arr, *(*arr).data.add(i));
        }
        new_arr
    }
}

/// Drop first n elements
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_drop(arr: *const NamlArray, n: i64) -> *mut NamlArray {
    unsafe {
        if arr.is_null() {
            return naml_array_new(0);
        }
        let skip = std::cmp::min(n as usize, (*arr).len);
        let remaining = (*arr).len - skip;
        let new_arr = naml_array_new(remaining);
        for i in skip..(*arr).len {
            naml_array_push(new_arr, *(*arr).data.add(i));
        }
        new_arr
    }
}

/// Slice array from start to end (exclusive)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_slice(arr: *const NamlArray, start: i64, end: i64) -> *mut NamlArray {
    unsafe {
        if arr.is_null() {
            return naml_array_new(0);
        }
        let len = (*arr).len;
        let start_idx = std::cmp::max(0, start) as usize;
        let end_idx = std::cmp::min(end as usize, len);
        if start_idx >= end_idx {
            return naml_array_new(0);
        }
        let slice_len = end_idx - start_idx;
        let new_arr = naml_array_new(slice_len);
        for i in start_idx..end_idx {
            naml_array_push(new_arr, *(*arr).data.add(i));
        }
        new_arr
    }
}

/// Find index of value (returns -1 if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_index_of(arr: *const NamlArray, value: i64) -> i64 {
    unsafe {
        if arr.is_null() {
            return -1;
        }
        for i in 0..(*arr).len {
            if *(*arr).data.add(i) == value {
                return i as i64;
            }
        }
        -1
    }
}

type PredicateFn = unsafe extern "C" fn(data_ptr: i64, element: i64) -> i64;
type MapperFn = unsafe extern "C" fn(data_ptr: i64, element: i64) -> i64;

/// Check if any element satisfies the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_any(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        return 0;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            return 1;
        }
    }
    0
}

/// Check if all elements satisfy the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_all(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        return 1;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) == 0 {
            return 0;
        }
    }
    1
}

/// Count elements satisfying the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_count(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        return 0;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    let mut count = 0i64;
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            count += 1;
        }
    }
    count
}

/// Map each element through a function, returning a new array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_map(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    if arr.is_null() || func_ptr == 0 {
        return naml_array_new(0);
    }
    let mapper: MapperFn = std::mem::transmute(func_ptr as usize);
    let len = (*arr).len;
    let new_arr = naml_array_new(len);
    for i in 0..len {
        let elem = *(*arr).data.add(i);
        let result = mapper(data_ptr, elem);
        naml_array_push(new_arr, result);
    }
    new_arr
}

/// Filter elements by predicate, returning a new array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_filter(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    if arr.is_null() || func_ptr == 0 {
        return naml_array_new(0);
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    let new_arr = naml_array_new(0);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            naml_array_push(new_arr, elem);
        }
    }
    new_arr
}

/// Find first element satisfying predicate (returns the element, -1 sentinel if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_find(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
    found_flag: *mut i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            if !found_flag.is_null() {
                *found_flag = 1;
            }
            return elem;
        }
    }
    if !found_flag.is_null() {
        *found_flag = 0;
    }
    0
}

/// Find index of first element satisfying predicate (returns -1 if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_find_index(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        return -1;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            return i as i64;
        }
    }
    -1
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
            naml_array_push(arr, 30); // Triggers growth

            assert_eq!(naml_array_len(arr), 3);
            assert_eq!(naml_array_get(arr, 0), 10);
            assert_eq!(naml_array_get(arr, 1), 20);
            assert_eq!(naml_array_get(arr, 2), 30);

            naml_array_decref(arr);
        }
    }

    #[test]
    fn test_array_from() {
        unsafe {
            let values = [1i64, 2, 3, 4, 5];
            let arr = naml_array_from(values.as_ptr(), values.len());

            assert_eq!(naml_array_len(arr), 5);
            for i in 0..5 {
                assert_eq!(naml_array_get(arr, i as i64), (i + 1) as i64);
            }

            naml_array_decref(arr);
        }
    }
}
