#![allow(unsafe_op_in_unsafe_fn)]
//!
//! naml-std-collections - Collection Helper Functions
//!
//! Provides array helper functions for naml programs:
//!
//! ## Access Functions
//! - `first(arr: [int]) -> option<int>` - Get first element
//! - `last(arr: [int]) -> option<int>` - Get last element
//!
//! ## Aggregation
//! - `sum(arr: [int]) -> int` - Sum all elements
//! - `min(arr: [int]) -> option<int>` - Find minimum
//! - `max(arr: [int]) -> option<int>` - Find maximum
//!
//! ## Transformation
//! - `reversed(arr: [int]) -> [int]` - Create reversed copy
//! - `take(arr: [int], n: int) -> [int]` - Take first n elements
//! - `drop(arr: [int], n: int) -> [int]` - Drop first n elements
//! - `slice(arr: [int], start: int, end: int) -> [int]` - Get slice
//!
//! ## Search
//! - `index_of(arr: [int], val: int) -> option<int>` - Find index of value
//! - `contains(arr: [int], val: int) -> bool` - Check if contains
//!
//! ## Lambda-based Functions
//! - `any(arr: [int], fn: fn(int) -> bool) -> bool` - Check if any match
//! - `all(arr: [int], fn: fn(int) -> bool) -> bool` - Check if all match
//! - `count(arr: [int], fn: fn(int) -> bool) -> int` - Count matches
//! - `apply(arr: [int], fn: fn(int) -> int) -> [int]` - Map function
//! - `where(arr: [int], fn: fn(int) -> bool) -> [int]` - Filter function
//! - `find(arr: [int], fn: fn(int) -> bool) -> option<int>` - Find first match
//! - `find_index(arr: [int], fn: fn(int) -> bool) -> option<int>` - Find index
//!
//! ## Advanced
//! - `fold(arr: [int], init: int, fn: fn(int, int) -> int) -> int` - Reduce
//! - `flatten(arr: [[int]]) -> [int]` - Flatten nested arrays
//! - `sort(arr: [int]) -> [int]` - Sort ascending
//! - `sort_by(arr: [int], fn: fn(int, int) -> int) -> [int]` - Sort with comparator
//!

use naml_std_core::{NamlArray, naml_array_new, naml_array_push};

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
type FoldFn = unsafe extern "C" fn(data_ptr: i64, accumulator: i64, element: i64) -> i64;
type CompareFn = unsafe extern "C" fn(data_ptr: i64, a: i64, b: i64) -> i64;

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

/// Map each element through a function, returning a new array (apply)
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

/// Filter elements by predicate, returning a new array (where)
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

/// Fold/reduce array with initial value and accumulator function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_fold(
    arr: *const NamlArray,
    initial: i64,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 {
        return initial;
    }
    let folder: FoldFn = std::mem::transmute(func_ptr as usize);
    let mut acc = initial;
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        acc = folder(data_ptr, acc, elem);
    }
    acc
}

/// Flatten nested array of arrays into a single array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_flatten(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() {
        return naml_array_new(0);
    }
    let new_arr = naml_array_new(0);
    for i in 0..(*arr).len {
        let inner_ptr = *(*arr).data.add(i) as *const NamlArray;
        if !inner_ptr.is_null() {
            for j in 0..(*inner_ptr).len {
                naml_array_push(new_arr, *(*inner_ptr).data.add(j));
            }
        }
    }
    new_arr
}

/// Sort array in place (ascending order)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_sort(arr: *mut NamlArray) -> *mut NamlArray {
    if arr.is_null() || (*arr).len <= 1 {
        return arr;
    }
    let slice = std::slice::from_raw_parts_mut((*arr).data, (*arr).len);
    slice.sort();
    arr
}

/// Sort array in place using a comparator function
/// Comparator should return < 0 if a < b, 0 if a == b, > 0 if a > b
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_sort_by(
    arr: *mut NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    if arr.is_null() || (*arr).len <= 1 || func_ptr == 0 {
        return arr;
    }
    let comparator: CompareFn = std::mem::transmute(func_ptr as usize);
    let slice = std::slice::from_raw_parts_mut((*arr).data, (*arr).len);
    slice.sort_by(|a, b| {
        let cmp = comparator(data_ptr, *a, *b);
        cmp.cmp(&0)
    });
    arr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum() {
        unsafe {
            let arr = naml_array_new(5);
            for i in 1..=5 {
                naml_array_push(arr, i);
            }
            assert_eq!(naml_array_sum(arr), 15);
        }
    }

    #[test]
    fn test_min_max() {
        unsafe {
            let arr = naml_array_new(5);
            naml_array_push(arr, 3);
            naml_array_push(arr, 1);
            naml_array_push(arr, 4);
            naml_array_push(arr, 1);
            naml_array_push(arr, 5);
            assert_eq!(naml_array_min(arr), 1);
            assert_eq!(naml_array_max(arr), 5);
        }
    }

    #[test]
    fn test_reversed() {
        unsafe {
            let arr = naml_array_new(3);
            naml_array_push(arr, 1);
            naml_array_push(arr, 2);
            naml_array_push(arr, 3);
            let rev = naml_array_reversed(arr);
            assert_eq!(*(*rev).data.add(0), 3);
            assert_eq!(*(*rev).data.add(1), 2);
            assert_eq!(*(*rev).data.add(2), 1);
        }
    }

    #[test]
    fn test_sort() {
        unsafe {
            let arr = naml_array_new(5);
            naml_array_push(arr, 3);
            naml_array_push(arr, 1);
            naml_array_push(arr, 4);
            naml_array_push(arr, 1);
            naml_array_push(arr, 5);
            naml_array_sort(arr);
            assert_eq!(*(*arr).data.add(0), 1);
            assert_eq!(*(*arr).data.add(1), 1);
            assert_eq!(*(*arr).data.add(2), 3);
            assert_eq!(*(*arr).data.add(3), 4);
            assert_eq!(*(*arr).data.add(4), 5);
        }
    }
}
