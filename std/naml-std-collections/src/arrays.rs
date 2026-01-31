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
//! ## Basic Functions
//! - `count(arr: [int]) -> int` - Get array length
//! - `push(arr: [int], val: int) -> unit` - Append element
//! - `pop(arr: [int]) -> option<int>` - Remove and return last element
//! - `shift(arr: [int]) -> option<int>` - Remove and return first element
//! - `fill(arr: [int], val: int) -> unit` - Fill array with value
//! - `clear(arr: [int]) -> unit` - Remove all elements
//! - `get(arr: [int], index: int) -> option<int>` - Safe index access
//!
//! ## Lambda-based Functions
//! - `any(arr: [int], fn: fn(int) -> bool) -> bool` - Check if any match
//! - `all(arr: [int], fn: fn(int) -> bool) -> bool` - Check if all match
//! - `count_if(arr: [int], fn: fn(int) -> bool) -> int` - Count matches
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
//! ## Mutation Operations
//! - `insert(arr: [int], index: int, value: int) -> unit` - Insert at index
//! - `remove_at(arr: [int], index: int) -> option<int>` - Remove at index
//! - `remove(arr: [int], value: int) -> bool` - Remove first occurrence
//! - `swap(arr: [int], i: int, j: int) -> unit` - Swap two elements
//!
//! ## Deduplication
//! - `unique(arr: [int]) -> [int]` - Remove duplicates preserving order
//! - `compact(arr: [int]) -> [int]` - Remove consecutive duplicates
//!
//! ## Backward Search
//! - `last_index_of(arr: [int], val: int) -> option<int>` - Find last index
//! - `find_last(arr: [int], fn: fn(int) -> bool) -> option<int>` - Find last match
//! - `find_last_index(arr: [int], fn: fn(int) -> bool) -> option<int>` - Find last index
//!
//! ## Array Combination
//! - `concat(arr1: [int], arr2: [int]) -> [int]` - Concatenate arrays
//! - `zip(arr1: [int], arr2: [int]) -> [[int]]` - Zip two arrays
//! - `unzip(arr: [[int]]) -> [[int]]` - Unzip array of pairs
//!
//! ## Splitting
//! - `chunk(arr: [int], size: int) -> [[int]]` - Split into chunks
//! - `partition(arr: [int], fn: fn(int) -> bool) -> [[int]]` - Partition by predicate
//!
//! ## Set Operations
//! - `intersect(arr1: [int], arr2: [int]) -> [int]` - Intersection
//! - `diff(arr1: [int], arr2: [int]) -> [int]` - Difference
//! - `union(arr1: [int], arr2: [int]) -> [int]` - Union
//!
//! ## Advanced Iteration
//! - `take_while(arr: [int], fn: fn(int) -> bool) -> [int]` - Take while predicate
//! - `drop_while(arr: [int], fn: fn(int) -> bool) -> [int]` - Drop while predicate
//! - `reject(arr: [int], fn: fn(int) -> bool) -> [int]` - Opposite of filter
//! - `flat_apply(arr: [int], fn: fn(int) -> [int]) -> [int]` - FlatMap
//! - `scan(arr: [int], init: int, fn: fn(int, int) -> int) -> [int]` - Running fold
//!
//! ## Random
//! - `shuffle(arr: [int]) -> [int]` - Shuffle array
//! - `sample(arr: [int]) -> option<int>` - Random element
//! - `sample_n(arr: [int], n: int) -> [int]` - Random n elements
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
pub unsafe extern "C" fn naml_array_count_if(
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

/// Insert element at index, shifting subsequent elements right
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_insert(arr: *mut NamlArray, index: i64, value: i64) {
    if arr.is_null() {
        return;
    }
    let len = (*arr).len;
    let idx = if index < 0 { 0 } else { std::cmp::min(index as usize, len) };
    if (*arr).len >= (*arr).capacity {
        let new_cap = if (*arr).capacity == 0 { 4 } else { (*arr).capacity * 2 };
        let new_data = std::alloc::realloc(
            (*arr).data as *mut u8,
            std::alloc::Layout::array::<i64>((*arr).capacity).unwrap(),
            new_cap * std::mem::size_of::<i64>(),
        ) as *mut i64;
        (*arr).data = new_data;
        (*arr).capacity = new_cap;
    }
    if idx < len {
        std::ptr::copy((*arr).data.add(idx), (*arr).data.add(idx + 1), len - idx);
    }
    *(*arr).data.add(idx) = value;
    (*arr).len += 1;
}

/// Remove element at index, returning the removed value (returns 0 if invalid)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_remove_at(arr: *mut NamlArray, index: i64) -> i64 {
    if arr.is_null() || index < 0 || index as usize >= (*arr).len {
        return 0;
    }
    let idx = index as usize;
    let value = *(*arr).data.add(idx);
    let len = (*arr).len;
    if idx < len - 1 {
        std::ptr::copy((*arr).data.add(idx + 1), (*arr).data.add(idx), len - idx - 1);
    }
    (*arr).len -= 1;
    value
}

/// Remove first occurrence of value, returning 1 if found and removed, 0 otherwise
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_remove(arr: *mut NamlArray, value: i64) -> i64 {
    if arr.is_null() {
        return 0;
    }
    for i in 0..(*arr).len {
        if *(*arr).data.add(i) == value {
            let len = (*arr).len;
            if i < len - 1 {
                std::ptr::copy((*arr).data.add(i + 1), (*arr).data.add(i), len - i - 1);
            }
            (*arr).len -= 1;
            return 1;
        }
    }
    0
}

/// Swap elements at indices i and j
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_swap(arr: *mut NamlArray, i: i64, j: i64) {
    if arr.is_null() {
        return;
    }
    let len = (*arr).len;
    if i < 0 || j < 0 || i as usize >= len || j as usize >= len {
        return;
    }
    let idx_i = i as usize;
    let idx_j = j as usize;
    let temp = *(*arr).data.add(idx_i);
    *(*arr).data.add(idx_i) = *(*arr).data.add(idx_j);
    *(*arr).data.add(idx_j) = temp;
}

/// Create new array with duplicates removed (preserving first occurrence order)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_unique(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() {
        return naml_array_new(0);
    }
    let new_arr = naml_array_new((*arr).len);
    for i in 0..(*arr).len {
        let val = *(*arr).data.add(i);
        let mut found = false;
        for j in 0..(*new_arr).len {
            if *(*new_arr).data.add(j) == val {
                found = true;
                break;
            }
        }
        if !found {
            naml_array_push(new_arr, val);
        }
    }
    new_arr
}

/// Create new array with falsy values (0) removed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_compact(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() || (*arr).len == 0 {
        return naml_array_new(0);
    }
    let new_arr = naml_array_new((*arr).len);
    for i in 0..(*arr).len {
        let val = *(*arr).data.add(i);
        if val != 0 {
            naml_array_push(new_arr, val);
        }
    }
    new_arr
}

/// Find last index of value (returns -1 if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_last_index_of(arr: *const NamlArray, value: i64) -> i64 {
    if arr.is_null() || (*arr).len == 0 {
        return -1;
    }
    for i in (0..(*arr).len).rev() {
        if *(*arr).data.add(i) == value {
            return i as i64;
        }
    }
    -1
}

/// Find last element satisfying predicate (returns element, sets found_flag)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_find_last(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
    found_flag: *mut i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 || (*arr).len == 0 {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in (0..(*arr).len).rev() {
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

/// Find index of last element satisfying predicate (returns -1 if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_find_last_index(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if arr.is_null() || func_ptr == 0 || (*arr).len == 0 {
        return -1;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in (0..(*arr).len).rev() {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            return i as i64;
        }
    }
    -1
}

/// Concatenate two arrays
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_concat(
    arr1: *const NamlArray,
    arr2: *const NamlArray,
) -> *mut NamlArray {
    let len1 = if arr1.is_null() { 0 } else { (*arr1).len };
    let len2 = if arr2.is_null() { 0 } else { (*arr2).len };
    let new_arr = naml_array_new(len1 + len2);
    for i in 0..len1 {
        naml_array_push(new_arr, *(*arr1).data.add(i));
    }
    for i in 0..len2 {
        naml_array_push(new_arr, *(*arr2).data.add(i));
    }
    new_arr
}

/// Zip two arrays into array of pairs (as 2-element arrays)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_zip(
    arr1: *const NamlArray,
    arr2: *const NamlArray,
) -> *mut NamlArray {
    let len1 = if arr1.is_null() { 0 } else { (*arr1).len };
    let len2 = if arr2.is_null() { 0 } else { (*arr2).len };
    let min_len = std::cmp::min(len1, len2);
    let result = naml_array_new(min_len);
    for i in 0..min_len {
        let pair = naml_array_new(2);
        naml_array_push(pair, *(*arr1).data.add(i));
        naml_array_push(pair, *(*arr2).data.add(i));
        naml_array_push(result, pair as i64);
    }
    result
}

/// Unzip array of pairs into array containing two arrays
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_unzip(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() || (*arr).len == 0 {
        let result = naml_array_new(2);
        naml_array_push(result, naml_array_new(0) as i64);
        naml_array_push(result, naml_array_new(0) as i64);
        return result;
    }
    let len = (*arr).len;
    let arr1 = naml_array_new(len);
    let arr2 = naml_array_new(len);
    for i in 0..len {
        let pair = *(*arr).data.add(i) as *const NamlArray;
        if !pair.is_null() && (*pair).len >= 2 {
            naml_array_push(arr1, *(*pair).data);
            naml_array_push(arr2, *(*pair).data.add(1));
        }
    }
    let result = naml_array_new(2);
    naml_array_push(result, arr1 as i64);
    naml_array_push(result, arr2 as i64);
    result
}

/// Split array into chunks of given size
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_chunk(arr: *const NamlArray, size: i64) -> *mut NamlArray {
    if arr.is_null() || size <= 0 {
        return naml_array_new(0);
    }
    let chunk_size = size as usize;
    let len = (*arr).len;
    let num_chunks = (len + chunk_size - 1) / chunk_size;
    let result = naml_array_new(num_chunks);
    let mut i = 0;
    while i < len {
        let end = std::cmp::min(i + chunk_size, len);
        let chunk = naml_array_new(end - i);
        for j in i..end {
            naml_array_push(chunk, *(*arr).data.add(j));
        }
        naml_array_push(result, chunk as i64);
        i = end;
    }
    result
}

/// Partition array by predicate into [matching, non-matching]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_partition(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    let matching = naml_array_new(0);
    let non_matching = naml_array_new(0);
    if !arr.is_null() && func_ptr != 0 {
        let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
        for i in 0..(*arr).len {
            let elem = *(*arr).data.add(i);
            if predicate(data_ptr, elem) != 0 {
                naml_array_push(matching, elem);
            } else {
                naml_array_push(non_matching, elem);
            }
        }
    }
    let result = naml_array_new(2);
    naml_array_push(result, matching as i64);
    naml_array_push(result, non_matching as i64);
    result
}

/// Intersection of two arrays
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_intersect(
    arr1: *const NamlArray,
    arr2: *const NamlArray,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if arr1.is_null() || arr2.is_null() {
        return result;
    }
    for i in 0..(*arr1).len {
        let val = *(*arr1).data.add(i);
        let mut in_arr2 = false;
        for j in 0..(*arr2).len {
            if *(*arr2).data.add(j) == val {
                in_arr2 = true;
                break;
            }
        }
        if in_arr2 {
            let mut already_added = false;
            for k in 0..(*result).len {
                if *(*result).data.add(k) == val {
                    already_added = true;
                    break;
                }
            }
            if !already_added {
                naml_array_push(result, val);
            }
        }
    }
    result
}

/// Difference of two arrays (elements in arr1 but not in arr2)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_diff(
    arr1: *const NamlArray,
    arr2: *const NamlArray,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if arr1.is_null() {
        return result;
    }
    for i in 0..(*arr1).len {
        let val = *(*arr1).data.add(i);
        let mut in_arr2 = false;
        if !arr2.is_null() {
            for j in 0..(*arr2).len {
                if *(*arr2).data.add(j) == val {
                    in_arr2 = true;
                    break;
                }
            }
        }
        if !in_arr2 {
            naml_array_push(result, val);
        }
    }
    result
}

/// Union of two arrays (unique elements from both)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_union(
    arr1: *const NamlArray,
    arr2: *const NamlArray,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if !arr1.is_null() {
        for i in 0..(*arr1).len {
            let val = *(*arr1).data.add(i);
            let mut found = false;
            for j in 0..(*result).len {
                if *(*result).data.add(j) == val {
                    found = true;
                    break;
                }
            }
            if !found {
                naml_array_push(result, val);
            }
        }
    }
    if !arr2.is_null() {
        for i in 0..(*arr2).len {
            let val = *(*arr2).data.add(i);
            let mut found = false;
            for j in 0..(*result).len {
                if *(*result).data.add(j) == val {
                    found = true;
                    break;
                }
            }
            if !found {
                naml_array_push(result, val);
            }
        }
    }
    result
}

/// Take elements while predicate is true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_take_while(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if arr.is_null() || func_ptr == 0 {
        return result;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) != 0 {
            naml_array_push(result, elem);
        } else {
            break;
        }
    }
    result
}

/// Drop elements while predicate is true
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_drop_while(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    if arr.is_null() || func_ptr == 0 {
        return naml_array_new(0);
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    let mut start = 0;
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) == 0 {
            start = i;
            break;
        }
        start = (*arr).len;
    }
    let result = naml_array_new((*arr).len - start);
    for i in start..(*arr).len {
        naml_array_push(result, *(*arr).data.add(i));
    }
    result
}

/// Reject elements matching predicate (opposite of filter)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_reject(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if arr.is_null() || func_ptr == 0 {
        return result;
    }
    let predicate: PredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        if predicate(data_ptr, elem) == 0 {
            naml_array_push(result, elem);
        }
    }
    result
}

type FlatMapFn = unsafe extern "C" fn(data_ptr: i64, element: i64) -> *mut NamlArray;

/// FlatMap - apply function returning array and flatten results
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_flat_apply(
    arr: *const NamlArray,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    let result = naml_array_new(0);
    if arr.is_null() || func_ptr == 0 {
        return result;
    }
    let mapper: FlatMapFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        let inner = mapper(data_ptr, elem);
        if !inner.is_null() {
            for j in 0..(*inner).len {
                naml_array_push(result, *(*inner).data.add(j));
            }
        }
    }
    result
}

/// Scan - running fold, returning array of intermediate results
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_scan(
    arr: *const NamlArray,
    initial: i64,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlArray {
    if arr.is_null() || func_ptr == 0 {
        return naml_array_new(0);
    }
    let folder: FoldFn = std::mem::transmute(func_ptr as usize);
    let result = naml_array_new((*arr).len);
    let mut acc = initial;
    for i in 0..(*arr).len {
        let elem = *(*arr).data.add(i);
        acc = folder(data_ptr, acc, elem);
        naml_array_push(result, acc);
    }
    result
}

/// Shuffle array (Fisher-Yates) - returns new shuffled array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_shuffle(arr: *const NamlArray) -> *mut NamlArray {
    if arr.is_null() || (*arr).len == 0 {
        return naml_array_new(0);
    }
    let len = (*arr).len;
    let result = naml_array_new(len);
    for i in 0..len {
        naml_array_push(result, *(*arr).data.add(i));
    }
    for i in (1..len).rev() {
        let j = naml_std_random::naml_random(0, i as i64) as usize;
        let temp = *(*result).data.add(i);
        *(*result).data.add(i) = *(*result).data.add(j);
        *(*result).data.add(j) = temp;
    }
    result
}

/// Sample random element from array (returns 0 and sets found_flag=0 if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_sample(arr: *const NamlArray, found_flag: *mut i64) -> i64 {
    if arr.is_null() || (*arr).len == 0 {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    if !found_flag.is_null() {
        *found_flag = 1;
    }
    let idx = naml_std_random::naml_random(0, ((*arr).len - 1) as i64) as usize;
    *(*arr).data.add(idx)
}

/// Sample n random elements from array (without replacement)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_array_sample_n(arr: *const NamlArray, n: i64) -> *mut NamlArray {
    if arr.is_null() || (*arr).len == 0 || n <= 0 {
        return naml_array_new(0);
    }
    let len = (*arr).len;
    let sample_count = std::cmp::min(n as usize, len);
    let shuffled = naml_array_shuffle(arr);
    let result = naml_array_new(sample_count);
    for i in 0..sample_count {
        naml_array_push(result, *(*shuffled).data.add(i));
    }
    result
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
