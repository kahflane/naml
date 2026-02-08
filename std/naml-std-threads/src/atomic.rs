///
/// Atomic Types for naml
///
/// Provides lock-free atomic primitives for concurrent programming.
/// Three types are supported:
/// - `atomic<int>` - 64-bit signed atomic integer
/// - `atomic<uint>` - 64-bit unsigned atomic integer
/// - `atomic<bool>` - atomic boolean
///
/// All operations use SeqCst ordering for safety and simplicity.
///
/// Usage in naml:
/// ```naml
/// var counter: atomic<int> = atomic(0);
/// atomic_add(counter, 10);
/// var val: int = atomic_load(counter);
/// ```
///

use std::alloc::{alloc, dealloc, Layout};
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicBool, Ordering};

use naml_std_core::{HeapHeader, HeapTag};

#[repr(C)]
pub struct NamlAtomicInt {
    pub header: HeapHeader,
    inner: AtomicI64,
}

#[repr(C)]
pub struct NamlAtomicUint {
    pub header: HeapHeader,
    inner: AtomicU64,
}

#[repr(C)]
pub struct NamlAtomicBool {
    pub header: HeapHeader,
    inner: AtomicBool,
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_atomic_int_new(value: i64) -> *mut NamlAtomicInt {
    unsafe {
        let layout = Layout::new::<NamlAtomicInt>();
        let ptr = alloc(layout) as *mut NamlAtomicInt;
        if ptr.is_null() {
            panic!("Failed to allocate atomic<int>");
        }
        std::ptr::write(ptr, NamlAtomicInt {
            header: HeapHeader::new(HeapTag::AtomicInt),
            inner: AtomicI64::new(value),
        });
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_load(ptr: *mut NamlAtomicInt) -> i64 {
    unsafe { (*ptr).inner.load(Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_store(ptr: *mut NamlAtomicInt, value: i64) {
    unsafe { (*ptr).inner.store(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_add(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_add(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_sub(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_sub(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_inc(ptr: *mut NamlAtomicInt) -> i64 {
    unsafe { (*ptr).inner.fetch_add(1, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_dec(ptr: *mut NamlAtomicInt) -> i64 {
    unsafe { (*ptr).inner.fetch_sub(1, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_cas(ptr: *mut NamlAtomicInt, expected: i64, new: i64) -> i64 {
    unsafe {
        match (*ptr).inner.compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_swap(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.swap(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_and(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_and(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_or(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_or(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_xor(ptr: *mut NamlAtomicInt, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_xor(value, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_incref(ptr: *mut NamlAtomicInt) {
    if !ptr.is_null() {
        unsafe { (*ptr).header.incref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_int_decref(ptr: *mut NamlAtomicInt) {
    if !ptr.is_null() {
        unsafe {
            if (*ptr).header.decref() {
                std::ptr::drop_in_place(ptr);
                let layout = Layout::new::<NamlAtomicInt>();
                dealloc(ptr as *mut u8, layout);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_atomic_uint_new(value: i64) -> *mut NamlAtomicUint {
    unsafe {
        let layout = Layout::new::<NamlAtomicUint>();
        let ptr = alloc(layout) as *mut NamlAtomicUint;
        if ptr.is_null() {
            panic!("Failed to allocate atomic<uint>");
        }
        std::ptr::write(ptr, NamlAtomicUint {
            header: HeapHeader::new(HeapTag::AtomicUint),
            inner: AtomicU64::new(value as u64),
        });
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_load(ptr: *mut NamlAtomicUint) -> i64 {
    unsafe { (*ptr).inner.load(Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_store(ptr: *mut NamlAtomicUint, value: i64) {
    unsafe { (*ptr).inner.store(value as u64, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_add(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_add(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_sub(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_sub(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_inc(ptr: *mut NamlAtomicUint) -> i64 {
    unsafe { (*ptr).inner.fetch_add(1, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_dec(ptr: *mut NamlAtomicUint) -> i64 {
    unsafe { (*ptr).inner.fetch_sub(1, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_cas(ptr: *mut NamlAtomicUint, expected: i64, new: i64) -> i64 {
    unsafe {
        match (*ptr).inner.compare_exchange(expected as u64, new as u64, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_swap(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.swap(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_and(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_and(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_or(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_or(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_xor(ptr: *mut NamlAtomicUint, value: i64) -> i64 {
    unsafe { (*ptr).inner.fetch_xor(value as u64, Ordering::SeqCst) as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_incref(ptr: *mut NamlAtomicUint) {
    if !ptr.is_null() {
        unsafe { (*ptr).header.incref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_uint_decref(ptr: *mut NamlAtomicUint) {
    if !ptr.is_null() {
        unsafe {
            if (*ptr).header.decref() {
                std::ptr::drop_in_place(ptr);
                let layout = Layout::new::<NamlAtomicUint>();
                dealloc(ptr as *mut u8, layout);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_atomic_bool_new(value: i64) -> *mut NamlAtomicBool {
    unsafe {
        let layout = Layout::new::<NamlAtomicBool>();
        let ptr = alloc(layout) as *mut NamlAtomicBool;
        if ptr.is_null() {
            panic!("Failed to allocate atomic<bool>");
        }
        std::ptr::write(ptr, NamlAtomicBool {
            header: HeapHeader::new(HeapTag::AtomicBool),
            inner: AtomicBool::new(value != 0),
        });
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_load(ptr: *mut NamlAtomicBool) -> i64 {
    unsafe { if (*ptr).inner.load(Ordering::SeqCst) { 1 } else { 0 } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_store(ptr: *mut NamlAtomicBool, value: i64) {
    unsafe { (*ptr).inner.store(value != 0, Ordering::SeqCst) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_cas(ptr: *mut NamlAtomicBool, expected: i64, new: i64) -> i64 {
    unsafe {
        match (*ptr).inner.compare_exchange(expected != 0, new != 0, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_swap(ptr: *mut NamlAtomicBool, value: i64) -> i64 {
    unsafe { if (*ptr).inner.swap(value != 0, Ordering::SeqCst) { 1 } else { 0 } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_incref(ptr: *mut NamlAtomicBool) {
    if !ptr.is_null() {
        unsafe { (*ptr).header.incref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_atomic_bool_decref(ptr: *mut NamlAtomicBool) {
    if !ptr.is_null() {
        unsafe {
            if (*ptr).header.decref() {
                std::ptr::drop_in_place(ptr);
                let layout = Layout::new::<NamlAtomicBool>();
                dealloc(ptr as *mut u8, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_atomic_int_basic() {
        unsafe {
            let a = naml_atomic_int_new(42);
            assert_eq!(naml_atomic_int_load(a), 42);

            naml_atomic_int_store(a, 100);
            assert_eq!(naml_atomic_int_load(a), 100);

            let old = naml_atomic_int_add(a, 10);
            assert_eq!(old, 100);
            assert_eq!(naml_atomic_int_load(a), 110);

            let old = naml_atomic_int_sub(a, 5);
            assert_eq!(old, 110);
            assert_eq!(naml_atomic_int_load(a), 105);

            naml_atomic_int_decref(a);
        }
    }

    #[test]
    fn test_atomic_int_cas() {
        unsafe {
            let a = naml_atomic_int_new(42);

            let ok = naml_atomic_int_cas(a, 42, 100);
            assert_eq!(ok, 1);
            assert_eq!(naml_atomic_int_load(a), 100);

            let ok = naml_atomic_int_cas(a, 42, 200);
            assert_eq!(ok, 0);
            assert_eq!(naml_atomic_int_load(a), 100);

            naml_atomic_int_decref(a);
        }
    }

    #[test]
    fn test_atomic_int_concurrent() {
        let a = naml_atomic_int_new(0);

        let handles: Vec<_> = (0..10).map(|_| {
            let a_ptr = a as usize;
            thread::spawn(move || {
                unsafe {
                    let a = a_ptr as *mut NamlAtomicInt;
                    for _ in 0..100 {
                        naml_atomic_int_add(a, 1);
                    }
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        unsafe {
            assert_eq!(naml_atomic_int_load(a), 1000);
            naml_atomic_int_decref(a);
        }
    }

    #[test]
    fn test_atomic_bool_basic() {
        unsafe {
            let b = naml_atomic_bool_new(0);
            assert_eq!(naml_atomic_bool_load(b), 0);

            naml_atomic_bool_store(b, 1);
            assert_eq!(naml_atomic_bool_load(b), 1);

            let ok = naml_atomic_bool_cas(b, 1, 0);
            assert_eq!(ok, 1);
            assert_eq!(naml_atomic_bool_load(b), 0);

            naml_atomic_bool_decref(b);
        }
    }
}
