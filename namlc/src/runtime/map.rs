//!
//! Map Runtime
//!
//! Hash map implementation for naml map<K, V> type.
//! Uses string keys with FNV-1a hashing and linear probing.
//!

use std::alloc::{alloc, alloc_zeroed, dealloc, Layout};
use super::value::{HeapHeader, HeapTag, NamlString, naml_string_decref};

const INITIAL_CAPACITY: usize = 16;
const LOAD_FACTOR: f64 = 0.75;

#[repr(C)]
pub struct NamlMap {
    pub header: HeapHeader,
    pub capacity: usize,
    pub length: usize,
    pub entries: *mut MapEntry,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MapEntry {
    pub key: i64,      // Pointer to NamlString or 0 if empty
    pub value: i64,    // The stored value
    pub occupied: bool,
}

impl Default for MapEntry {
    fn default() -> Self {
        Self { key: 0, value: 0, occupied: false }
    }
}

fn hash_string(s: *const NamlString) -> u64 {
    if s.is_null() { return 0; }
    unsafe {
        let len = (*s).len;
        let data = (*s).data.as_ptr();
        // FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        for i in 0..len {
            hash ^= *data.add(i) as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

fn string_eq(a: *const NamlString, b: *const NamlString) -> bool {
    if a.is_null() && b.is_null() { return true; }
    if a.is_null() || b.is_null() { return false; }
    unsafe {
        if (*a).len != (*b).len { return false; }
        let a_slice = std::slice::from_raw_parts((*a).data.as_ptr(), (*a).len);
        let b_slice = std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len);
        a_slice == b_slice
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_new(capacity: usize) -> *mut NamlMap {
    let cap = if capacity < INITIAL_CAPACITY { INITIAL_CAPACITY } else { capacity };
    unsafe {
        let map_layout = Layout::new::<NamlMap>();
        let map_ptr = alloc(map_layout) as *mut NamlMap;
        if map_ptr.is_null() { panic!("Failed to allocate map"); }

        let entries_layout = Layout::array::<MapEntry>(cap).unwrap();
        let entries_ptr = alloc_zeroed(entries_layout) as *mut MapEntry;
        if entries_ptr.is_null() { panic!("Failed to allocate map entries"); }

        (*map_ptr).header = HeapHeader::new(HeapTag::Map);
        (*map_ptr).capacity = cap;
        (*map_ptr).length = 0;
        (*map_ptr).entries = entries_ptr;
        map_ptr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_set(map: *mut NamlMap, key: i64, value: i64) {
    if map.is_null() { return; }
    unsafe {
        if ((*map).length + 1) as f64 / (*map).capacity as f64 > LOAD_FACTOR {
            resize_map(map);
        }
        let hash = hash_string(key as *const NamlString);
        let mut idx = (hash as usize) % (*map).capacity;
        loop {
            let entry = (*map).entries.add(idx);
            if !(*entry).occupied {
                (*entry).key = key;
                (*entry).value = value;
                (*entry).occupied = true;
                (*map).length += 1;
                if key != 0 { (*(key as *mut NamlString)).header.incref(); }
                return;
            }
            if string_eq((*entry).key as *const NamlString, key as *const NamlString) {
                (*entry).value = value;
                return;
            }
            idx = (idx + 1) % (*map).capacity;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_get(map: *const NamlMap, key: i64) -> i64 {
    if map.is_null() { return 0; }
    unsafe {
        let hash = hash_string(key as *const NamlString);
        let mut idx = (hash as usize) % (*map).capacity;
        let start_idx = idx;
        loop {
            let entry = (*map).entries.add(idx);
            if !(*entry).occupied { return 0; }
            if string_eq((*entry).key as *const NamlString, key as *const NamlString) {
                return (*entry).value;
            }
            idx = (idx + 1) % (*map).capacity;
            if idx == start_idx { break; }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_contains(map: *const NamlMap, key: i64) -> i64 {
    if map.is_null() { return 0; }
    unsafe {
        let hash = hash_string(key as *const NamlString);
        let mut idx = (hash as usize) % (*map).capacity;
        let start_idx = idx;
        loop {
            let entry = (*map).entries.add(idx);
            if !(*entry).occupied { return 0; }
            if string_eq((*entry).key as *const NamlString, key as *const NamlString) {
                return 1;
            }
            idx = (idx + 1) % (*map).capacity;
            if idx == start_idx { break; }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_len(map: *const NamlMap) -> i64 {
    if map.is_null() { 0 } else { unsafe { (*map).length as i64 } }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_incref(map: *mut NamlMap) {
    if !map.is_null() { unsafe { (*map).header.incref(); } }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_map_decref(map: *mut NamlMap) {
    if map.is_null() { return; }
    unsafe {
        if (*map).header.decref() {
            for i in 0..(*map).capacity {
                let entry = (*map).entries.add(i);
                if (*entry).occupied && (*entry).key != 0 {
                    naml_string_decref((*entry).key as *mut NamlString);
                }
            }
            let entries_layout = Layout::array::<MapEntry>((*map).capacity).unwrap();
            dealloc((*map).entries as *mut u8, entries_layout);
            let map_layout = Layout::new::<NamlMap>();
            dealloc(map as *mut u8, map_layout);
        }
    }
}

/// Decrement map reference count and also decref string values
#[unsafe(no_mangle)]
pub extern "C" fn naml_map_decref_strings(map: *mut NamlMap) {
    if map.is_null() { return; }
    unsafe {
        if (*map).header.decref() {
            for i in 0..(*map).capacity {
                let entry = (*map).entries.add(i);
                if (*entry).occupied {
                    if (*entry).key != 0 {
                        naml_string_decref((*entry).key as *mut NamlString);
                    }
                    if (*entry).value != 0 {
                        naml_string_decref((*entry).value as *mut NamlString);
                    }
                }
            }
            let entries_layout = Layout::array::<MapEntry>((*map).capacity).unwrap();
            dealloc((*map).entries as *mut u8, entries_layout);
            let map_layout = Layout::new::<NamlMap>();
            dealloc(map as *mut u8, map_layout);
        }
    }
}

/// Decrement map reference count and also decref array values
#[unsafe(no_mangle)]
pub extern "C" fn naml_map_decref_arrays(map: *mut NamlMap) {
    if map.is_null() { return; }
    unsafe {
        if (*map).header.decref() {
            for i in 0..(*map).capacity {
                let entry = (*map).entries.add(i);
                if (*entry).occupied {
                    if (*entry).key != 0 {
                        naml_string_decref((*entry).key as *mut NamlString);
                    }
                    if (*entry).value != 0 {
                        super::array::naml_array_decref((*entry).value as *mut super::array::NamlArray);
                    }
                }
            }
            let entries_layout = Layout::array::<MapEntry>((*map).capacity).unwrap();
            dealloc((*map).entries as *mut u8, entries_layout);
            let map_layout = Layout::new::<NamlMap>();
            dealloc(map as *mut u8, map_layout);
        }
    }
}

/// Decrement map reference count and also decref nested map values
#[unsafe(no_mangle)]
pub extern "C" fn naml_map_decref_maps(map: *mut NamlMap) {
    if map.is_null() { return; }
    unsafe {
        if (*map).header.decref() {
            for i in 0..(*map).capacity {
                let entry = (*map).entries.add(i);
                if (*entry).occupied {
                    if (*entry).key != 0 {
                        naml_string_decref((*entry).key as *mut NamlString);
                    }
                    if (*entry).value != 0 {
                        naml_map_decref((*entry).value as *mut NamlMap);
                    }
                }
            }
            let entries_layout = Layout::array::<MapEntry>((*map).capacity).unwrap();
            dealloc((*map).entries as *mut u8, entries_layout);
            let map_layout = Layout::new::<NamlMap>();
            dealloc(map as *mut u8, map_layout);
        }
    }
}

/// Decrement map reference count and also decref struct values
#[unsafe(no_mangle)]
pub extern "C" fn naml_map_decref_structs(map: *mut NamlMap) {
    if map.is_null() { return; }
    unsafe {
        if (*map).header.decref() {
            for i in 0..(*map).capacity {
                let entry = (*map).entries.add(i);
                if (*entry).occupied {
                    if (*entry).key != 0 {
                        naml_string_decref((*entry).key as *mut NamlString);
                    }
                    if (*entry).value != 0 {
                        super::value::naml_struct_decref((*entry).value as *mut super::value::NamlStruct);
                    }
                }
            }
            let entries_layout = Layout::array::<MapEntry>((*map).capacity).unwrap();
            dealloc((*map).entries as *mut u8, entries_layout);
            let map_layout = Layout::new::<NamlMap>();
            dealloc(map as *mut u8, map_layout);
        }
    }
}

unsafe fn resize_map(map: *mut NamlMap) {
    unsafe {
        let old_capacity = (*map).capacity;
        let old_entries = (*map).entries;
        let new_capacity = old_capacity * 2;

        let new_layout = Layout::array::<MapEntry>(new_capacity).unwrap();
        let new_entries = alloc_zeroed(new_layout) as *mut MapEntry;
        if new_entries.is_null() { panic!("Failed to resize map"); }

        (*map).entries = new_entries;
        (*map).capacity = new_capacity;
        (*map).length = 0;

        for i in 0..old_capacity {
            let entry = old_entries.add(i);
            if (*entry).occupied {
                // Use internal rehash function that doesn't modify reference counts.
                // We're moving entries to new locations - the map still owns the same
                // references, so refcounts should remain unchanged.
                rehash_entry(map, (*entry).key, (*entry).value);
            }
        }

        let old_layout = Layout::array::<MapEntry>(old_capacity).unwrap();
        dealloc(old_entries as *mut u8, old_layout);
    }
}

/// Internal function to insert an entry during rehashing without modifying reference counts.
/// Used only during resize when moving existing entries to new locations.
unsafe fn rehash_entry(map: *mut NamlMap, key: i64, value: i64) {
    unsafe {
        let hash = hash_string(key as *const NamlString);
        let mut idx = (hash as usize) % (*map).capacity;
        loop {
            let entry = (*map).entries.add(idx);
            if !(*entry).occupied {
                (*entry).key = key;
                (*entry).value = value;
                (*entry).occupied = true;
                (*map).length += 1;
                // No incref here - we're just moving the entry, not creating a new reference
                return;
            }
            idx = (idx + 1) % (*map).capacity;
        }
    }
}
