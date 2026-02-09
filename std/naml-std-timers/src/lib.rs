///
/// naml-std-timers — Non-blocking timer and interval utilities
///
/// Provides set_timeout, cancel_timeout, set_interval, cancel_interval.
/// Uses a single background timer thread with a priority-sorted queue.
/// When a timer fires, its callback is dispatched to the M:N scheduler
/// thread pool via `naml_spawn_closure`.
///
/// ## Timer Thread Design
///
/// A lazy-initialized background thread maintains a sorted list of pending
/// timers. It sleeps until the next timer fires (via `Condvar::wait_timeout`),
/// then dispatches expired callbacks. For intervals, the timer is re-queued
/// with an updated fire time after each dispatch.
///
/// ## Closure Data Lifetime
///
/// The scheduler's worker loop frees closure data after the callback runs.
/// For one-shot timeouts, the timer thread passes the original data pointer.
/// For intervals, the timer thread copies the closure data before each
/// dispatch, keeping the original for re-use.
///
/// ## Thread Safety
///
/// All state is behind a `Mutex` + `Condvar`. Timer IDs are generated from
/// an `AtomicU64` counter. The cancel set uses `HashSet<u64>`.
///

use std::alloc::{Layout, alloc};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use naml_std_threads::naml_spawn_closure;

type TaskFn = extern "C" fn(*mut u8);

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

struct TimerEntry {
    id: u64,
    fire_at: Instant,
    func: TaskFn,
    data_ptr: *mut u8,
    data_size: usize,
    interval_ms: Option<u64>,
}

unsafe impl Send for TimerEntry {}

struct TimerState {
    timers: Vec<TimerEntry>,
    cancelled: HashSet<u64>,
}

impl TimerState {
    fn new() -> Self {
        Self {
            timers: Vec::new(),
            cancelled: HashSet::new(),
        }
    }

    fn insert(&mut self, entry: TimerEntry) {
        let pos = self
            .timers
            .binary_search_by(|e| e.fire_at.cmp(&entry.fire_at))
            .unwrap_or_else(|pos| pos);
        self.timers.insert(pos, entry);
    }
}

struct TimerManager {
    state: Mutex<TimerState>,
    condvar: Condvar,
}

impl TimerManager {
    fn new() -> Self {
        let manager = Self {
            state: Mutex::new(TimerState::new()),
            condvar: Condvar::new(),
        };

        std::thread::spawn(|| {
            timer_thread_loop();
        });

        manager
    }

    fn add_timer(
        &self,
        func: TaskFn,
        data_ptr: *mut u8,
        data_size: usize,
        delay_ms: u64,
        interval_ms: Option<u64>,
    ) -> u64 {
        let id = NEXT_TIMER_ID.fetch_add(1, Ordering::Relaxed);
        let entry = TimerEntry {
            id,
            fire_at: Instant::now() + Duration::from_millis(delay_ms),
            func,
            data_ptr,
            data_size,
            interval_ms,
        };

        let mut state = self.state.lock().unwrap();
        state.insert(entry);
        self.condvar.notify_one();

        id
    }

    fn cancel(&self, id: u64) {
        let mut state = self.state.lock().unwrap();
        state.cancelled.insert(id);
        state.timers.retain(|t| t.id != id);
        self.condvar.notify_one();
    }
}

static TIMER_MANAGER: OnceLock<TimerManager> = OnceLock::new();

fn get_timer_manager() -> &'static TimerManager {
    TIMER_MANAGER.get_or_init(TimerManager::new)
}

fn timer_thread_loop() {
    let manager = get_timer_manager();

    loop {
        let mut state = manager.state.lock().unwrap();

        if state.timers.is_empty() {
            state = manager.condvar.wait(state).unwrap();
        }

        if state.timers.is_empty() {
            continue;
        }

        let next_fire = state.timers[0].fire_at;
        let now = Instant::now();

        if next_fire > now {
            let wait = next_fire - now;
            let (new_state, _) = manager.condvar.wait_timeout(state, wait).unwrap();
            state = new_state;
        }

        let now = Instant::now();
        let mut to_fire = Vec::new();
        let mut to_requeue = Vec::new();

        while let Some(entry) = state.timers.first() {
            if entry.fire_at > now {
                break;
            }
            let entry = state.timers.remove(0);
            if state.cancelled.contains(&entry.id) {
                state.cancelled.remove(&entry.id);
                if !entry.data_ptr.is_null() && entry.data_size > 0 {
                    unsafe {
                        let layout = Layout::from_size_align_unchecked(entry.data_size, 8);
                        std::alloc::dealloc(entry.data_ptr, layout);
                    }
                }
                continue;
            }
            to_fire.push(entry);
        }

        drop(state);

        for entry in to_fire {
            if entry.interval_ms.is_some() {
                let data_copy = copy_closure_data(entry.data_ptr, entry.data_size);
                naml_spawn_closure(
                    entry.func,
                    data_copy,
                    entry.data_size,
                );

                let interval = entry.interval_ms.unwrap();
                to_requeue.push(TimerEntry {
                    id: entry.id,
                    fire_at: Instant::now() + Duration::from_millis(interval),
                    func: entry.func,
                    data_ptr: entry.data_ptr,
                    data_size: entry.data_size,
                    interval_ms: entry.interval_ms,
                });
            } else {
                naml_spawn_closure(
                    entry.func,
                    entry.data_ptr,
                    entry.data_size,
                );
            }
        }

        if !to_requeue.is_empty() {
            let mut state = manager.state.lock().unwrap();
            for entry in to_requeue {
                if !state.cancelled.contains(&entry.id) {
                    state.insert(entry);
                } else {
                    state.cancelled.remove(&entry.id);
                    if !entry.data_ptr.is_null() && entry.data_size > 0 {
                        unsafe {
                            let layout =
                                Layout::from_size_align_unchecked(entry.data_size, 8);
                            std::alloc::dealloc(entry.data_ptr, layout);
                        }
                    }
                }
            }
        }
    }
}

fn copy_closure_data(src: *mut u8, size: usize) -> *mut u8 {
    if src.is_null() || size == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = Layout::from_size_align_unchecked(size, 8);
        let dst = alloc(layout);
        std::ptr::copy_nonoverlapping(src, dst, size);
        dst
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_set_timeout(
    func_ptr: i64,
    data_ptr: i64,
    data_size: i64,
    delay_ms: i64,
) -> i64 {
    let func: TaskFn = unsafe { std::mem::transmute(func_ptr) };
    let data = data_ptr as *mut u8;
    let size = data_size as usize;
    let delay = if delay_ms < 0 { 0 } else { delay_ms as u64 };

    get_timer_manager().add_timer(func, data, size, delay, None) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_cancel_timeout(handle: i64) {
    get_timer_manager().cancel(handle as u64);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_set_interval(
    func_ptr: i64,
    data_ptr: i64,
    data_size: i64,
    interval_ms: i64,
) -> i64 {
    let func: TaskFn = unsafe { std::mem::transmute(func_ptr) };
    let data = data_ptr as *mut u8;
    let size = data_size as usize;
    let interval = if interval_ms < 1 { 1 } else { interval_ms as u64 };

    get_timer_manager().add_timer(func, data, size, interval, Some(interval)) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_cancel_interval(handle: i64) {
    get_timer_manager().cancel(handle as u64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    static TEST_COUNTER: AtomicI64 = AtomicI64::new(0);

    extern "C" fn increment_counter(_data: *mut u8) {
        TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_set_timeout_fires() {
        TEST_COUNTER.store(0, Ordering::SeqCst);

        let handle = naml_timers_set_timeout(
            increment_counter as *const () as i64,
            std::ptr::null_mut::<u8>() as i64,
            0,
            50,
        );
        assert!(handle > 0);

        std::thread::sleep(Duration::from_millis(200));
        naml_std_threads::naml_wait_all();

        assert_eq!(TEST_COUNTER.load(Ordering::SeqCst), 1);
    }

    static CANCEL_COUNTER: AtomicI64 = AtomicI64::new(0);

    extern "C" fn increment_cancel(_data: *mut u8) {
        CANCEL_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_cancel_timeout() {
        CANCEL_COUNTER.store(0, Ordering::SeqCst);

        let handle = naml_timers_set_timeout(
            increment_cancel as *const () as i64,
            std::ptr::null_mut::<u8>() as i64,
            0,
            200,
        );
        naml_timers_cancel_timeout(handle);

        std::thread::sleep(Duration::from_millis(400));
        naml_std_threads::naml_wait_all();

        assert_eq!(CANCEL_COUNTER.load(Ordering::SeqCst), 0);
    }

    static INTERVAL_COUNTER: AtomicI64 = AtomicI64::new(0);

    extern "C" fn increment_interval(_data: *mut u8) {
        INTERVAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_set_interval_fires_multiple() {
        INTERVAL_COUNTER.store(0, Ordering::SeqCst);

        let handle = naml_timers_set_interval(
            increment_interval as *const () as i64,
            std::ptr::null_mut::<u8>() as i64,
            0,
            50,
        );

        std::thread::sleep(Duration::from_millis(280));
        naml_timers_cancel_interval(handle);

        std::thread::sleep(Duration::from_millis(100));
        naml_std_threads::naml_wait_all();

        let count = INTERVAL_COUNTER.load(Ordering::SeqCst);
        assert!(count >= 3, "Expected at least 3 ticks, got {}", count);
    }
}
