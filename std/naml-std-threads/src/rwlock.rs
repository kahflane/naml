//!
//! RwLock Implementation for naml
//!
//! Provides a reader-writer lock for protecting shared data with multiple
//! concurrent readers or exclusive writers.
//!
//! Usage in naml:
//! ```naml
//! var rw: rwlock<int> = with_rwlock(0);
//!
//! // Read lock (multiple readers can hold simultaneously)
//! rlocked (value: int in rw) {
//!     print(value);
//! }
//!
//! // Write lock (exclusive access)
//! wlocked (value: int in rw) {
//!     value = value + 1;
//! }
//! ```
//!

use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use naml_std_core::{HeapHeader, HeapTag};

enum RwLockGuard {
    Read(RwLockReadGuard<'static, i64>),
    Write(RwLockWriteGuard<'static, i64>),
}

thread_local! {
    static ACTIVE_RW_GUARDS: RefCell<HashMap<usize, RwLockGuard>> = RefCell::new(HashMap::new());
}

#[repr(C)]
pub struct NamlRwLock {
    pub header: HeapHeader,
    inner: RwLock<i64>,
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_rwlock_new(initial_value: i64) -> *mut NamlRwLock {
    unsafe {
        let layout = Layout::new::<NamlRwLock>();
        let ptr = alloc(layout) as *mut NamlRwLock;
        if ptr.is_null() {
            panic!("Failed to allocate rwlock");
        }

        std::ptr::write(ptr, NamlRwLock {
            header: HeapHeader::new(HeapTag::Rwlock),
            inner: RwLock::new(initial_value),
        });

        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_incref(rw: *mut NamlRwLock) {
    if !rw.is_null() {
        unsafe { (*rw).header.incref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_decref(rw: *mut NamlRwLock) {
    if !rw.is_null() {
        unsafe {
            if (*rw).header.decref() {
                std::ptr::drop_in_place(rw);
                let layout = Layout::new::<NamlRwLock>();
                dealloc(rw as *mut u8, layout);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_read_lock(rw: *mut NamlRwLock) -> i64 {
    if rw.is_null() {
        return 0;
    }

    unsafe {
        let rwlock = &*rw;
        let guard = rwlock.inner.read().unwrap();
        let value = *guard;

        // Store the guard in thread-local storage
        let guard: RwLockReadGuard<'static, i64> = std::mem::transmute(guard);
        ACTIVE_RW_GUARDS.with(|guards| {
            guards.borrow_mut().insert(rw as usize, RwLockGuard::Read(guard));
        });

        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_read_unlock(rw: *mut NamlRwLock) {
    if rw.is_null() {
        return;
    }

    // Retrieve and drop the guard from thread-local storage
    ACTIVE_RW_GUARDS.with(|guards| {
        guards.borrow_mut().remove(&(rw as usize));
        // Guard is dropped here, releasing the lock
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_write_lock(rw: *mut NamlRwLock) -> i64 {
    if rw.is_null() {
        return 0;
    }

    unsafe {
        let rwlock = &*rw;
        let guard = rwlock.inner.write().unwrap();
        let value = *guard;

        // Store the guard in thread-local storage
        let guard: RwLockWriteGuard<'static, i64> = std::mem::transmute(guard);
        ACTIVE_RW_GUARDS.with(|guards| {
            guards.borrow_mut().insert(rw as usize, RwLockGuard::Write(guard));
        });

        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_write_unlock(rw: *mut NamlRwLock, new_value: i64) {
    if rw.is_null() {
        return;
    }

    // Retrieve and drop the guard from thread-local storage
    ACTIVE_RW_GUARDS.with(|guards| {
        if let Some(guard) = guards.borrow_mut().remove(&(rw as usize)) {
            if let RwLockGuard::Write(mut write_guard) = guard {
                // Update the value before releasing the lock
                *write_guard = new_value;
                // Guard is dropped here, releasing the lock
            }
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_get(rw: *mut NamlRwLock) -> i64 {
    if rw.is_null() {
        return 0;
    }

    unsafe {
        let rwlock = &*rw;
        let guard = rwlock.inner.read().unwrap();
        *guard
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_set(rw: *mut NamlRwLock, new_value: i64) {
    if rw.is_null() {
        return;
    }

    unsafe {
        let rwlock = &*rw;
        let mut guard = rwlock.inner.write().unwrap();
        *guard = new_value;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_try_read_lock(rw: *mut NamlRwLock, out_value: *mut i64) -> i64 {
    if rw.is_null() {
        return 0;
    }

    unsafe {
        let rwlock = &*rw;
        match rwlock.inner.try_read() {
            Ok(guard) => {
                if !out_value.is_null() {
                    *out_value = *guard;
                }
                let guard: RwLockReadGuard<'static, i64> = std::mem::transmute(guard);
                ACTIVE_RW_GUARDS.with(|guards| {
                    guards.borrow_mut().insert(rw as usize, RwLockGuard::Read(guard));
                });
                1
            }
            Err(_) => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_rwlock_try_write_lock(rw: *mut NamlRwLock, out_value: *mut i64) -> i64 {
    if rw.is_null() {
        return 0;
    }

    unsafe {
        let rwlock = &*rw;
        match rwlock.inner.try_write() {
            Ok(guard) => {
                if !out_value.is_null() {
                    *out_value = *guard;
                }
                let guard: RwLockWriteGuard<'static, i64> = std::mem::transmute(guard);
                ACTIVE_RW_GUARDS.with(|guards| {
                    guards.borrow_mut().insert(rw as usize, RwLockGuard::Write(guard));
                });
                1
            }
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_rwlock_basic() {
        unsafe {
            let rw = naml_rwlock_new(42);
            assert!(!rw.is_null());

            let value = naml_rwlock_read_lock(rw);
            assert_eq!(value, 42);
            naml_rwlock_read_unlock(rw);

            let value = naml_rwlock_write_lock(rw);
            assert_eq!(value, 42);
            naml_rwlock_write_unlock(rw, 100);

            let new_value = naml_rwlock_get(rw);
            assert_eq!(new_value, 100);

            naml_rwlock_decref(rw);
        }
    }

    #[test]
    fn test_rwlock_concurrent_readers() {
        let rw = naml_rwlock_new(42);
        let reader_count = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..5).map(|_| {
            let rw_ptr = rw as usize;
            let count = Arc::clone(&reader_count);
            thread::spawn(move || {
                unsafe {
                    let rw = rw_ptr as *mut NamlRwLock;
                    let value = naml_rwlock_read_lock(rw);
                    count.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(value, 42);
                    thread::sleep(std::time::Duration::from_millis(10));
                    count.fetch_sub(1, Ordering::SeqCst);
                    naml_rwlock_read_unlock(rw);
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        unsafe { naml_rwlock_decref(rw); }
    }

    #[test]
    fn test_rwlock_writer_exclusive() {
        let rw = naml_rwlock_new(0);

        let handles: Vec<_> = (0..5).map(|_| {
            let rw_ptr = rw as usize;
            thread::spawn(move || {
                unsafe {
                    let rw = rw_ptr as *mut NamlRwLock;
                    for _ in 0..100 {
                        let value = naml_rwlock_write_lock(rw);
                        naml_rwlock_write_unlock(rw, value + 1);
                    }
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        unsafe {
            let final_value = naml_rwlock_get(rw);
            assert_eq!(final_value, 500);
            naml_rwlock_decref(rw);
        }
    }
}
