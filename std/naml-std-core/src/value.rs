//!
//! Runtime Value Representation
//!
//! naml values at runtime are represented as 64-bit values that can be either:
//! - Inline primitives (int, float, bool) stored directly
//! - Heap pointers to reference-counted objects (strings, arrays, structs)
//!
//! We use NaN-boxing for efficient representation:
//! - If the high bits indicate NaN, the low bits contain a pointer or tag
//! - Otherwise, the value is a valid f64
//!
//! For simplicity in Phase 2, we use a simpler tagged pointer scheme:
//! - Bit 0: 0 = pointer, 1 = immediate
//! - For immediates, bits 1-3 encode the type
//!

use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{alloc, dealloc, Layout};

/// Type tags for heap objects
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapTag {
    String = 0,
    Array = 1,
    Struct = 2,
    Map = 3,
    Closure = 4,
    Channel = 5,
    Bytes = 6,
    Mutex = 7,
    Rwlock = 8,
    Json = 9,
    AtomicInt = 10,
    AtomicUint = 11,
    AtomicBool = 12,
}

/// Header for all heap-allocated objects
#[repr(C)]
pub struct HeapHeader {
    pub refcount: AtomicUsize,
    pub tag: HeapTag,
    pub _pad: [u8; 7],
}

impl HeapHeader {
    pub fn new(tag: HeapTag) -> Self {
        Self {
            refcount: AtomicUsize::new(1),
            tag,
            _pad: [0; 7],
        }
    }

    pub fn incref(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decref(&self) -> bool {
        if self.refcount.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            true
        } else {
            false
        }
    }

    pub fn refcount(&self) -> usize {
        self.refcount.load(Ordering::Relaxed)
    }
}

/// A heap-allocated string
#[repr(C)]
pub struct NamlString {
    pub header: HeapHeader,
    pub len: usize,
    pub data: [u8; 0],
}

impl NamlString {
    pub fn as_str(&self) -> &str {
        unsafe {
            let slice = std::slice::from_raw_parts(self.data.as_ptr(), self.len);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

/// A heap-allocated struct instance
#[repr(C)]
pub struct NamlStruct {
    pub header: HeapHeader,
    pub type_id: u32,
    pub field_count: u32,
    pub fields: [i64; 0],
}

/// Allocate a new string on the heap
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_new(data: *const u8, len: usize) -> *mut NamlString {
    unsafe {
        let layout = Layout::from_size_align(
            std::mem::size_of::<NamlString>() + len,
            std::mem::align_of::<NamlString>(),
        ).unwrap();

        let ptr = alloc(layout) as *mut NamlString;
        if ptr.is_null() {
            panic!("Failed to allocate string");
        }

        (*ptr).header = HeapHeader::new(HeapTag::String);
        (*ptr).len = len;

        if !data.is_null() && len > 0 {
            std::ptr::copy_nonoverlapping(data, (*ptr).data.as_mut_ptr(), len);
        }

        ptr
    }
}

/// Increment reference count of a string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_incref(s: *mut NamlString) {
    if !s.is_null() {
        unsafe { (*s).header.incref(); }
    }
}

/// Decrement reference count and free if zero
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_decref(s: *mut NamlString) {
    if !s.is_null() {
        unsafe {
            if (*s).header.decref() {
                let len = (*s).len;
                let layout = Layout::from_size_align(
                    std::mem::size_of::<NamlString>() + len,
                    std::mem::align_of::<NamlString>(),
                ).unwrap();
                dealloc(s as *mut u8, layout);
            }
        }
    }
}

/// Get string length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_len(s: *const NamlString) -> i64 {
    if s.is_null() {
        0
    } else {
        unsafe { (*s).len as i64 }
    }
}

/// Get pointer to string data (for printing)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_data(s: *const NamlString) -> *const u8 {
    if s.is_null() {
        std::ptr::null()
    } else {
        unsafe { (*s).data.as_ptr() }
    }
}

/// Concatenate two strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_concat(a: *const NamlString, b: *const NamlString) -> *mut NamlString {
    unsafe {
        let a_len = if a.is_null() { 0 } else { (*a).len };
        let b_len = if b.is_null() { 0 } else { (*b).len };
        let total_len = a_len + b_len;

        let result = naml_string_new(std::ptr::null(), total_len);

        if a_len > 0 {
            std::ptr::copy_nonoverlapping((*a).data.as_ptr(), (*result).data.as_mut_ptr(), a_len);
        }
        if b_len > 0 {
            std::ptr::copy_nonoverlapping((*b).data.as_ptr(), (*result).data.as_mut_ptr().add(a_len), b_len);
        }

        result
    }
}

/// Compare two strings for equality
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_eq(a: *const NamlString, b: *const NamlString) -> i64 {
    unsafe {
        if a.is_null() && b.is_null() {
            return 1;
        }
        if a.is_null() || b.is_null() {
            return 0;
        }
        if (*a).len != (*b).len {
            return 0;
        }

        let a_slice = std::slice::from_raw_parts((*a).data.as_ptr(), (*a).len);
        let b_slice = std::slice::from_raw_parts((*b).data.as_ptr(), (*b).len);

        if a_slice == b_slice { 1 } else { 0 }
    }
}

/// Print a NamlString (for debugging)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_print(s: *const NamlString) {
    if !s.is_null() {
        unsafe {
            let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
            if let Ok(str_val) = std::str::from_utf8(slice) {
                print!("{}", str_val);
            }
        }
    }
}

/// Create a NamlString from a null-terminated C string pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_from_cstr(cstr: *const i8) -> *mut NamlString {
    if cstr.is_null() {
        return unsafe { naml_string_new(std::ptr::null(), 0) };
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(cstr);
        let bytes = c_str.to_bytes();
        naml_string_new(bytes.as_ptr(), bytes.len())
    }
}

/// Convert an integer to a string
#[unsafe(no_mangle)]
pub extern "C" fn naml_int_to_string(n: i64) -> *mut NamlString {
    let s = n.to_string();
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

/// Convert a float to a string
#[unsafe(no_mangle)]
pub extern "C" fn naml_float_to_string(f: f64) -> *mut NamlString {
    let s = f.to_string();
    unsafe { naml_string_new(s.as_ptr(), s.len()) }
}

/// Convert a string to an integer (returns 0 on parse failure)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_to_int(s: *const NamlString) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        let str_val = (*s).as_str();
        str_val.parse::<i64>().unwrap_or(0)
    }
}

/// Convert a string to a float (returns 0.0 on parse failure)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_to_float(s: *const NamlString) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    unsafe {
        let str_val = (*s).as_str();
        str_val.parse::<f64>().unwrap_or(0.0)
    }
}

/// Try to convert a string to an integer (fallible)
/// Returns 1 if successful and writes result to out_value, 0 if failed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_try_to_int(s: *const NamlString, out_value: *mut i64) -> i64 {
    if s.is_null() || out_value.is_null() {
        return 0;
    }
    unsafe {
        let str_val = (*s).as_str();
        match str_val.trim().parse::<i64>() {
            Ok(v) => {
                *out_value = v;
                1
            }
            Err(_) => 0,
        }
    }
}

/// Try to convert a string to a float (fallible)
/// Returns 1 if successful and writes result to out_value, 0 if failed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_try_to_float(s: *const NamlString, out_value: *mut f64) -> i64 {
    if s.is_null() || out_value.is_null() {
        return 0;
    }
    unsafe {
        let str_val = (*s).as_str();
        match str_val.trim().parse::<f64>() {
            Ok(v) => {
                *out_value = v;
                1
            }
            Err(_) => 0,
        }
    }
}

/// Get character (as UTF-8 codepoint) at index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_char_at(s: *const NamlString, index: i64) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        let str_val = (*s).as_str();
        if let Some(c) = str_val.chars().nth(index as usize) {
            c as i64
        } else {
            0
        }
    }
}

/// Get string length in characters (not bytes)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_char_len(s: *const NamlString) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        (*s).as_str().chars().count() as i64
    }
}

/// Check if string is empty
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_is_empty(s: *const NamlString) -> i64 {
    if s.is_null() {
        return 1;
    }
    unsafe {
        if (*s).len == 0 { 1 } else { 0 }
    }
}

/// Trim whitespace from both ends of string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_string_trim(s: *const NamlString) -> *mut NamlString {
    unsafe {
        if s.is_null() {
            return naml_string_new(std::ptr::null(), 0);
        }
        let str_val = (*s).as_str();
        let trimmed = str_val.trim();
        naml_string_new(trimmed.as_ptr(), trimmed.len())
    }
}

/// Allocate a new struct on the heap using arena allocation
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_new(type_id: u32, field_count: u32) -> *mut NamlStruct {
    unsafe {
        let size = crate::arena::struct_alloc_size(field_count);
        let ptr = crate::arena::arena_alloc(size) as *mut NamlStruct;
        if ptr.is_null() {
            panic!("Failed to allocate struct");
        }

        // Only write refcount and metadata; skip pad zeroing and field zeroing.
        // The caller (StructLiteral codegen) always writes all fields.
        std::ptr::write(&mut (*ptr).header.refcount, AtomicUsize::new(1));
        std::ptr::write(&mut (*ptr).header.tag, HeapTag::Struct);
        (*ptr).type_id = type_id;
        (*ptr).field_count = field_count;

        ptr
    }
}

/// Increment reference count of a struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_incref(s: *mut NamlStruct) {
    if !s.is_null() {
        unsafe { (*s).header.incref(); }
    }
}

/// Decrement reference count and free if zero (for structs with no heap fields)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_decref(s: *mut NamlStruct) {
    if !s.is_null() {
        unsafe {
            if (*s).header.decref() {
                let field_count = (*s).field_count;
                let size = crate::arena::struct_alloc_size(field_count);
                crate::arena::arena_free(s as *mut u8, size);
            }
        }
    }
}

/// Increment reference count of a struct (non-atomic, for single-threaded --unsafe mode)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_incref_fast(s: *mut NamlStruct) {
    if !s.is_null() {
        unsafe {
            let rc = (*s).header.refcount.as_ptr();
            *rc += 1;
        }
    }
}

/// Decrement reference count and free if zero (non-atomic, for single-threaded --unsafe mode)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_decref_fast(s: *mut NamlStruct) {
    if !s.is_null() {
        unsafe {
            let rc = (*s).header.refcount.as_ptr();
            let old = *rc;
            *rc = old - 1;
            if old == 1 {
                let field_count = (*s).field_count;
                let size = crate::arena::struct_alloc_size(field_count);
                crate::arena::arena_free(s as *mut u8, size);
            }
        }
    }
}

/// Free struct memory without refcount check (called by generated decref functions)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_free(s: *mut NamlStruct) {
    if !s.is_null() {
        unsafe {
            let field_count = (*s).field_count;
            let size = crate::arena::struct_alloc_size(field_count);
            crate::arena::arena_free(s as *mut u8, size);
        }
    }
}

/// Iterative decref for self-recursive structs (e.g., tree nodes).
/// Uses a fixed-size stack on the call stack to avoid heap allocation.
/// For a balanced binary tree of depth d, max worklist size is d+1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_decref_iterative(
    ptr: *mut NamlStruct,
    heap_field_indices: *const u32,
    num_heap_fields: u32,
) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let rc = (*ptr).header.refcount.load(std::sync::atomic::Ordering::Relaxed);
        if rc > 1 {
            (*ptr).header.refcount.store(rc - 1, std::sync::atomic::Ordering::Relaxed);
            return;
        }

        const STACK_CAP: usize = 128;
        let mut stack: [*mut NamlStruct; STACK_CAP] = [std::ptr::null_mut(); STACK_CAP];
        let mut top: usize = 1;
        stack[0] = ptr;

        while top > 0 {
            top -= 1;
            let node = stack[top];

            let fields_base = (node as *const u8).add(24) as *const i64;
            for i in 0..num_heap_fields {
                let field_idx = *heap_field_indices.add(i as usize);
                let child = *fields_base.add(field_idx as usize) as *mut NamlStruct;
                if !child.is_null() {
                    let child_rc =
                        (*child).header.refcount.load(std::sync::atomic::Ordering::Relaxed);
                    if child_rc > 1 {
                        (*child)
                            .header
                            .refcount
                            .store(child_rc - 1, std::sync::atomic::Ordering::Relaxed);
                    } else if top < STACK_CAP {
                        stack[top] = child;
                        top += 1;
                    } else {
                        naml_struct_decref_iterative(child, heap_field_indices, num_heap_fields);
                    }
                }
            }

            let field_count = (*node).field_count;
            let size = crate::arena::struct_alloc_size(field_count);
            crate::arena::arena_free(node as *mut u8, size);
        }
    }
}

/// Get field value by index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_get_field(s: *const NamlStruct, field_index: u32) -> i64 {
    if s.is_null() {
        return 0;
    }

    unsafe {
        if field_index >= (*s).field_count {
            return 0;
        }
        *(*s).fields.as_ptr().add(field_index as usize)
    }
}

/// Set field value by index
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_struct_set_field(s: *mut NamlStruct, field_index: u32, value: i64) {
    if s.is_null() {
        return;
    }

    unsafe {
        if field_index < (*s).field_count {
            *(*s).fields.as_mut_ptr().add(field_index as usize) = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_creation() {
        unsafe {
            let data = b"hello";
            let s = naml_string_new(data.as_ptr(), data.len());
            assert!(!s.is_null());
            assert_eq!((*s).len, 5);
            assert_eq!((*s).header.refcount(), 1);
            naml_string_decref(s);
        }
    }

    #[test]
    fn test_string_concat() {
        unsafe {
            let a = naml_string_new(b"hello ".as_ptr(), 6);
            let b = naml_string_new(b"world".as_ptr(), 5);
            let c = naml_string_concat(a, b);

            assert_eq!((*c).len, 11);
            assert_eq!((*c).as_str(), "hello world");

            naml_string_decref(a);
            naml_string_decref(b);
            naml_string_decref(c);
        }
    }
}
