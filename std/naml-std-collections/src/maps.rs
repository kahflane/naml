#![allow(unsafe_op_in_unsafe_fn)]

///
/// Map Collection Functions
///
/// Provides map helper functions for naml programs.
/// All map functions operate on heap-allocated NamlMap structures.
///
/// ## Basic Operations
/// - `count(m) -> int` - Number of entries
/// - `contains_key(m, key) -> bool` - Check key exists
/// - `remove(m, key) -> option<V>` - Remove and return value
/// - `clear(m)` - Remove all entries
///
/// ## Extraction
/// - `keys(m) -> [K]` - Get all keys as array
/// - `values(m) -> [V]` - Get all values as array
/// - `entries(m) -> [[K,V]]` - Get key-value pairs
///
/// ## Transformation
/// - `transform(m, fn) -> map<K,U>` - Transform values
///
/// ## Filtering
/// - `where(m, fn) -> map<K,V>` - Keep matching entries
/// - `reject(m, fn) -> map<K,V>` - Remove matching entries
///
/// ## Combining
/// - `merge(a, b) -> map<K,V>` - Combine (b overwrites a)
/// - `defaults(m, defs) -> map<K,V>` - Fill missing from defs
///
/// ## Aggregation
/// - `fold(m, init, fn) -> R` - Reduce to single value
/// - `any(m, fn) -> bool` - Any entry matches
/// - `all(m, fn) -> bool` - All entries match
///

use naml_std_core::{NamlArray, NamlString, NamlMap,
                    naml_array_new, naml_array_push,
                    naml_map_new, naml_map_set, naml_map_contains,
                    hash_string, string_eq};

/// Get number of entries in map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_count(map: *const NamlMap) -> i64 {
    if map.is_null() {
        return 0;
    }
    (*map).length as i64
}

/// Check if map contains a key
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_contains_key(map: *const NamlMap, key: i64) -> i64 {
    if map.is_null() {
        return 0;
    }
    let hash = hash_string(key as *const NamlString);
    let mut idx = (hash as usize) % (*map).capacity;
    let start_idx = idx;
    loop {
        let entry = (*map).entries.add(idx);
        if !(*entry).occupied {
            return 0;
        }
        if string_eq((*entry).key as *const NamlString, key as *const NamlString) {
            return 1;
        }
        idx = (idx + 1) % (*map).capacity;
        if idx == start_idx {
            break;
        }
    }
    0
}

/// Remove entry by key and return the value (returns 0 if not found)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_remove(map: *mut NamlMap, key: i64, found_flag: *mut i64) -> i64 {
    if map.is_null() {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    let hash = hash_string(key as *const NamlString);
    let mut idx = (hash as usize) % (*map).capacity;
    let start_idx = idx;
    loop {
        let entry = (*map).entries.add(idx);
        if !(*entry).occupied {
            if !found_flag.is_null() {
                *found_flag = 0;
            }
            return 0;
        }
        if string_eq((*entry).key as *const NamlString, key as *const NamlString) {
            let value = (*entry).value;
            (*entry).occupied = false;
            (*entry).key = 0;
            (*entry).value = 0;
            (*map).length -= 1;
            if !found_flag.is_null() {
                *found_flag = 1;
            }
            return value;
        }
        idx = (idx + 1) % (*map).capacity;
        if idx == start_idx {
            break;
        }
    }
    if !found_flag.is_null() {
        *found_flag = 0;
    }
    0
}

/// Clear all entries from map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_clear(map: *mut NamlMap) {
    if map.is_null() {
        return;
    }
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            (*entry).occupied = false;
            (*entry).key = 0;
            (*entry).value = 0;
        }
    }
    (*map).length = 0;
}

/// Get all keys as array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_keys(map: *const NamlMap) -> *mut NamlArray {
    if map.is_null() {
        return naml_array_new(0);
    }
    let result = naml_array_new((*map).length);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            naml_array_push(result, (*entry).key);
        }
    }
    result
}

/// Get all values as array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_values(map: *const NamlMap) -> *mut NamlArray {
    if map.is_null() {
        return naml_array_new(0);
    }
    let result = naml_array_new((*map).length);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            naml_array_push(result, (*entry).value);
        }
    }
    result
}

/// Get all entries as array of [key, value] pairs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_entries(map: *const NamlMap) -> *mut NamlArray {
    if map.is_null() {
        return naml_array_new(0);
    }
    let result = naml_array_new((*map).length);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            let pair = naml_array_new(2);
            naml_array_push(pair, (*entry).key);
            naml_array_push(pair, (*entry).value);
            naml_array_push(result, pair as i64);
        }
    }
    result
}

/// Get first key (returns 0 if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_first_key(map: *const NamlMap, found_flag: *mut i64) -> i64 {
    if map.is_null() || (*map).length == 0 {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if !found_flag.is_null() {
                *found_flag = 1;
            }
            return (*entry).key;
        }
    }
    if !found_flag.is_null() {
        *found_flag = 0;
    }
    0
}

/// Get first value (returns 0 if empty)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_first_value(map: *const NamlMap, found_flag: *mut i64) -> i64 {
    if map.is_null() || (*map).length == 0 {
        if !found_flag.is_null() {
            *found_flag = 0;
        }
        return 0;
    }
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if !found_flag.is_null() {
                *found_flag = 1;
            }
            return (*entry).value;
        }
    }
    if !found_flag.is_null() {
        *found_flag = 0;
    }
    0
}

type MapPredicateFn = unsafe extern "C" fn(data_ptr: i64, key: i64, value: i64) -> i64;
type MapTransformFn = unsafe extern "C" fn(data_ptr: i64, value: i64) -> i64;
type MapFoldFn = unsafe extern "C" fn(data_ptr: i64, acc: i64, key: i64, value: i64) -> i64;

/// Check if any entry satisfies the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_any(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if map.is_null() || func_ptr == 0 {
        return 0;
    }
    let predicate: MapPredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if predicate(data_ptr, (*entry).key, (*entry).value) != 0 {
                return 1;
            }
        }
    }
    0
}

/// Check if all entries satisfy the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_all(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if map.is_null() || func_ptr == 0 {
        return 1;
    }
    if (*map).length == 0 {
        return 1;
    }
    let predicate: MapPredicateFn = std::mem::transmute(func_ptr as usize);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if predicate(data_ptr, (*entry).key, (*entry).value) == 0 {
                return 0;
            }
        }
    }
    1
}

/// Count entries satisfying the predicate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_count_if(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if map.is_null() || func_ptr == 0 {
        return 0;
    }
    let predicate: MapPredicateFn = std::mem::transmute(func_ptr as usize);
    let mut count = 0i64;
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if predicate(data_ptr, (*entry).key, (*entry).value) != 0 {
                count += 1;
            }
        }
    }
    count
}

/// Fold/reduce map with initial value and accumulator function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_fold(
    map: *const NamlMap,
    initial: i64,
    func_ptr: i64,
    data_ptr: i64,
) -> i64 {
    if map.is_null() || func_ptr == 0 {
        return initial;
    }
    let folder: MapFoldFn = std::mem::transmute(func_ptr as usize);
    let mut acc = initial;
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            acc = folder(data_ptr, acc, (*entry).key, (*entry).value);
        }
    }
    acc
}

/// Transform map values, returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_transform(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlMap {

    if map.is_null() || func_ptr == 0 {
        return naml_map_new(16);
    }
    let transformer: MapTransformFn = std::mem::transmute(func_ptr as usize);
    let result = naml_map_new((*map).capacity);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            let new_value = transformer(data_ptr, (*entry).value);
            naml_map_set(result, (*entry).key, new_value);
        }
    }
    result
}

/// Filter map entries by predicate, returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_where(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlMap {

    if map.is_null() || func_ptr == 0 {
        return naml_map_new(16);
    }
    let predicate: MapPredicateFn = std::mem::transmute(func_ptr as usize);
    let result = naml_map_new((*map).capacity);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if predicate(data_ptr, (*entry).key, (*entry).value) != 0 {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }
    result
}

/// Reject map entries by predicate (opposite of where), returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_reject(
    map: *const NamlMap,
    func_ptr: i64,
    data_ptr: i64,
) -> *mut NamlMap {

    if map.is_null() || func_ptr == 0 {
        return naml_map_new(16);
    }
    let predicate: MapPredicateFn = std::mem::transmute(func_ptr as usize);
    let result = naml_map_new((*map).capacity);
    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            if predicate(data_ptr, (*entry).key, (*entry).value) == 0 {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }
    result
}

/// Merge two maps (b overwrites a), returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_merge(
    a: *const NamlMap,
    b: *const NamlMap,
) -> *mut NamlMap {

    let cap_a = if a.is_null() { 0 } else { (*a).capacity };
    let cap_b = if b.is_null() { 0 } else { (*b).capacity };
    let result = naml_map_new(std::cmp::max(cap_a, cap_b).max(16));

    if !a.is_null() {
        for i in 0..(*a).capacity {
            let entry = (*a).entries.add(i);
            if (*entry).occupied {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    if !b.is_null() {
        for i in 0..(*b).capacity {
            let entry = (*b).entries.add(i);
            if (*entry).occupied {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    result
}

/// Fill missing keys from defaults, returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_defaults(
    map: *const NamlMap,
    defs: *const NamlMap,
) -> *mut NamlMap {

    let cap_m = if map.is_null() { 0 } else { (*map).capacity };
    let cap_d = if defs.is_null() { 0 } else { (*defs).capacity };
    let result = naml_map_new(std::cmp::max(cap_m, cap_d).max(16));

    if !defs.is_null() {
        for i in 0..(*defs).capacity {
            let entry = (*defs).entries.add(i);
            if (*entry).occupied {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    if !map.is_null() {
        for i in 0..(*map).capacity {
            let entry = (*map).entries.add(i);
            if (*entry).occupied {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    result
}

/// Get intersection of two maps (keys in both), returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_intersect(
    a: *const NamlMap,
    b: *const NamlMap,
) -> *mut NamlMap {

    if a.is_null() || b.is_null() {
        return naml_map_new(16);
    }

    let result = naml_map_new(std::cmp::min((*a).capacity, (*b).capacity).max(16));

    for i in 0..(*a).capacity {
        let entry = (*a).entries.add(i);
        if (*entry).occupied {
            if naml_map_contains(b, (*entry).key) != 0 {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    result
}

/// Get difference of two maps (keys in a but not in b), returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_diff(
    a: *const NamlMap,
    b: *const NamlMap,
) -> *mut NamlMap {

    if a.is_null() {
        return naml_map_new(16);
    }

    let result = naml_map_new((*a).capacity.max(16));

    for i in 0..(*a).capacity {
        let entry = (*a).entries.add(i);
        if (*entry).occupied {
            let in_b = if b.is_null() { 0 } else { naml_map_contains(b, (*entry).key) };
            if in_b == 0 {
                naml_map_set(result, (*entry).key, (*entry).value);
            }
        }
    }

    result
}

/// Invert map (swap keys and values), returning a new map
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_invert(map: *const NamlMap) -> *mut NamlMap {

    if map.is_null() {
        return naml_map_new(16);
    }

    let result = naml_map_new((*map).capacity);

    for i in 0..(*map).capacity {
        let entry = (*map).entries.add(i);
        if (*entry).occupied {
            naml_map_set(result, (*entry).value, (*entry).key);
        }
    }

    result
}

/// Create map from parallel key and value arrays
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_from_arrays(
    keys: *const NamlArray,
    values: *const NamlArray,
) -> *mut NamlMap {

    if keys.is_null() || values.is_null() {
        return naml_map_new(16);
    }

    let len = std::cmp::min((*keys).len, (*values).len);
    let result = naml_map_new(len.max(16));

    for i in 0..len {
        let key = *(*keys).data.add(i);
        let value = *(*values).data.add(i);
        naml_map_set(result, key, value);
    }

    result
}

/// Create map from array of [key, value] pairs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_map_from_entries(pairs: *const NamlArray) -> *mut NamlMap {

    if pairs.is_null() {
        return naml_map_new(16);
    }

    let result = naml_map_new((*pairs).len.max(16));

    for i in 0..(*pairs).len {
        let pair = *(*pairs).data.add(i) as *const NamlArray;
        if !pair.is_null() && (*pair).len >= 2 {
            let key = *(*pair).data;
            let value = *(*pair).data.add(1);
            naml_map_set(result, key, value);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_count() {
        unsafe {
            let map = naml_map_new(16);
            assert_eq!(naml_map_count(map), 0);
        }
    }

    #[test]
    fn test_map_keys_values() {
        unsafe {
            let map = naml_map_new(16);
            let keys = naml_map_keys(map);
            let values = naml_map_values(map);
            assert_eq!((*keys).len, 0);
            assert_eq!((*values).len, 0);
        }
    }
}
