//!
//! Mutex Implementation for naml
//!
//! Provides a mutual exclusion primitive for protecting shared data.
//! The mutex wraps a single i64 value (all naml values are i64 at runtime).
//!
//! Usage in naml:
//! ```naml
//! var m: mutex<int> = with_mutex(0);
//! locked (value: int in m) {
//!     value = value + 1;
//! }
//! ```
//!

use std::alloc::{alloc, dealloc, Layout};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use naml_std_core::{HeapHeader, HeapTag};

thread_local! {
    static ACTIVE_GUARDS: RefCell<HashMap<usize, MutexGuard<'static, i64>>> = RefCell::new(HashMap::new());
}

#[repr(C)]
pub struct NamlMutex {
    pub header: HeapHeader,
    inner: Mutex<i64>,
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_mutex_new(initial_value: i64) -> *mut NamlMutex {
    unsafe {
        let layout = Layout::new::<NamlMutex>();
        let ptr = alloc(layout) as *mut NamlMutex;
        if ptr.is_null() {
            panic!("Failed to allocate mutex");
        }

        std::ptr::write(ptr, NamlMutex {
            header: HeapHeader::new(HeapTag::Mutex),
            inner: Mutex::new(initial_value),
        });

        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_incref(m: *mut NamlMutex) {
    if !m.is_null() {
        unsafe { (*m).header.incref(); }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_decref(m: *mut NamlMutex) {
    if !m.is_null() {
        unsafe {
            if (*m).header.decref() {
                std::ptr::drop_in_place(m);
                let layout = Layout::new::<NamlMutex>();
                dealloc(m as *mut u8, layout);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_lock(m: *mut NamlMutex) -> i64 {
    if m.is_null() {
        return 0;
    }

    unsafe {
        let mutex = &*m;
        let guard = mutex.inner.lock().unwrap();
        let value = *guard;

        // Store the guard in thread-local storage
        // We transmute to 'static lifetime since we manage the lifetime manually
        let guard: MutexGuard<'static, i64> = std::mem::transmute(guard);
        ACTIVE_GUARDS.with(|guards| {
            guards.borrow_mut().insert(m as usize, guard);
        });

        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_unlock(m: *mut NamlMutex, new_value: i64) {
    if m.is_null() {
        return;
    }

    // Retrieve and drop the guard from thread-local storage
    ACTIVE_GUARDS.with(|guards| {
        if let Some(mut guard) = guards.borrow_mut().remove(&(m as usize)) {
            // Update the value before releasing the lock
            *guard = new_value;
            // Guard is dropped here, releasing the lock
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_get(m: *mut NamlMutex) -> i64 {
    if m.is_null() {
        return 0;
    }

    unsafe {
        let mutex = &*m;
        let guard = mutex.inner.lock().unwrap();
        *guard
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_set(m: *mut NamlMutex, new_value: i64) {
    if m.is_null() {
        return;
    }

    unsafe {
        let mutex = &*m;
        let mut guard = mutex.inner.lock().unwrap();
        *guard = new_value;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_mutex_try_lock(m: *mut NamlMutex, out_value: *mut i64) -> i64 {
    if m.is_null() {
        return 0;
    }

    unsafe {
        let mutex = &*m;
        match mutex.inner.try_lock() {
            Ok(guard) => {
                if !out_value.is_null() {
                    *out_value = *guard;
                }
                // Store the guard for later unlock
                let guard: MutexGuard<'static, i64> = std::mem::transmute(guard);
                ACTIVE_GUARDS.with(|guards| {
                    guards.borrow_mut().insert(m as usize, guard);
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

    #[test]
    fn test_mutex_basic() {
        unsafe {
            let m = naml_mutex_new(42);
            assert!(!m.is_null());

            let value = naml_mutex_lock(m);
            assert_eq!(value, 42);

            naml_mutex_unlock(m, 100);
            let new_value = naml_mutex_get(m);
            assert_eq!(new_value, 100);

            naml_mutex_decref(m);
        }
    }

    #[test]
    fn test_mutex_concurrent() {
        let m = naml_mutex_new(0);

        let handles: Vec<_> = (0..10).map(|_| {
            let m_ptr = m as usize;
            thread::spawn(move || {
                unsafe {
                    let m = m_ptr as *mut NamlMutex;
                    for _ in 0..100 {
                        let value = naml_mutex_lock(m);
                        naml_mutex_unlock(m, value + 1);
                    }
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }

        unsafe {
            let final_value = naml_mutex_get(m);
            assert_eq!(final_value, 1000);
            naml_mutex_decref(m);
        }
    }
}
