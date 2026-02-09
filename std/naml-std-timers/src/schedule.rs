///
/// Cron-style Job Scheduling
///
/// Provides cron expression-based scheduling using the `croner` crate.
/// A background scheduler thread maintains a map of active cron entries
/// and dispatches callbacks to the M:N scheduler thread pool when they fire.
///
/// ## Cron Expression Format
///
/// Standard 5-field (minute, hour, day-of-month, month, day-of-week)
/// or 6-field with optional seconds prefix:
///
/// ```text
/// ┌───────────── second (0-59) [optional]
/// │ ┌───────────── minute (0-59)
/// │ │ ┌───────────── hour (0-23)
/// │ │ │ ┌───────────── day of month (1-31)
/// │ │ │ │ ┌───────────── month (1-12)
/// │ │ │ │ │ ┌───────────── day of week (0-6, Sun=0)
/// │ │ │ │ │ │
/// * * * * * *
/// ```
///
/// Supports L, #, W modifiers via the croner crate.
///
/// ## Thread Safety
///
/// All state is behind a `Mutex` + `Condvar`. Entry IDs are generated
/// from an `AtomicU64` counter. Callbacks are dispatched via
/// `naml_spawn_closure` which copies closure data before each dispatch.
///
/// ## Exception Handling
///
/// Invalid cron expressions throw `ScheduleError` with a descriptive
/// message. Uses naml's typed exception mechanism.
///

use std::alloc::{Layout, alloc};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use chrono::Utc;
use croner::Cron;

use naml_std_core::{
    NamlString, naml_string_new, naml_stack_capture,
    naml_exception_set_typed, EXCEPTION_TYPE_SCHEDULE_ERROR,
};
use naml_std_threads::naml_spawn_closure;

type TaskFn = extern "C" fn(*mut u8);

static NEXT_CRON_ID: AtomicU64 = AtomicU64::new(1);

struct CronEntry {
    #[allow(dead_code)]
    id: u64,
    cron: Cron,
    func: TaskFn,
    data_ptr: *mut u8,
    data_size: usize,
    next_fire_ms: i64,
}

unsafe impl Send for CronEntry {}

struct CronState {
    entries: HashMap<u64, CronEntry>,
}

impl CronState {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn earliest_fire_ms(&self) -> Option<i64> {
        self.entries.values().map(|e| e.next_fire_ms).min()
    }
}

struct CronScheduler {
    state: Mutex<CronState>,
    condvar: Condvar,
}

impl CronScheduler {
    fn new() -> Self {
        let scheduler = Self {
            state: Mutex::new(CronState::new()),
            condvar: Condvar::new(),
        };

        std::thread::spawn(|| {
            cron_thread_loop();
        });

        scheduler
    }
}

static CRON_SCHEDULER: OnceLock<CronScheduler> = OnceLock::new();

fn get_cron_scheduler() -> &'static CronScheduler {
    CRON_SCHEDULER.get_or_init(CronScheduler::new)
}

fn calculate_next_fire_ms(cron: &Cron) -> Option<i64> {
    let now = Utc::now();
    cron.find_next_occurrence(&now, false)
        .ok()
        .map(|dt| dt.timestamp_millis())
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

fn throw_schedule_error(message: &str) {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let layout = Layout::from_size_align(16, 8).unwrap();
        let ptr = alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate ScheduleError");
        }
        *(ptr as *mut i64) = message_ptr as i64;
        let stack = naml_stack_capture();
        *(ptr.add(8) as *mut *mut u8) = stack;
        naml_exception_set_typed(ptr, EXCEPTION_TYPE_SCHEDULE_ERROR);
    }
}

fn cron_thread_loop() {
    let scheduler = get_cron_scheduler();

    loop {
        let mut state = scheduler.state.lock().unwrap();

        while state.entries.is_empty() {
            state = scheduler.condvar.wait(state).unwrap();
        }

        let earliest = state.earliest_fire_ms();
        if earliest.is_none() {
            continue;
        }
        let earliest_ms = earliest.unwrap();

        let now_ms = Utc::now().timestamp_millis();
        if earliest_ms > now_ms {
            let wait = Duration::from_millis((earliest_ms - now_ms) as u64);
            let (new_state, _) = scheduler.condvar.wait_timeout(state, wait).unwrap();
            state = new_state;
        }

        let now_ms = Utc::now().timestamp_millis();
        let mut to_fire: Vec<u64> = Vec::new();

        for (id, entry) in state.entries.iter() {
            if entry.next_fire_ms <= now_ms {
                to_fire.push(*id);
            }
        }

        let mut fire_data: Vec<(TaskFn, *mut u8, usize)> = Vec::new();
        for id in &to_fire {
            if let Some(entry) = state.entries.get_mut(id) {
                let data_copy = copy_closure_data(entry.data_ptr, entry.data_size);
                fire_data.push((entry.func, data_copy, entry.data_size));

                if let Some(next) = calculate_next_fire_ms(&entry.cron) {
                    entry.next_fire_ms = next;
                } else {
                    entry.next_fire_ms = now_ms + 60_000;
                }
            }
        }

        drop(state);

        for (func, data, size) in fire_data {
            naml_spawn_closure(func, data, size);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_schedule(
    func_ptr: i64,
    data_ptr: i64,
    data_size: i64,
    cron_expr: i64,
) -> i64 {
    let expr_ptr = cron_expr as *const NamlString;
    if expr_ptr.is_null() {
        throw_schedule_error("cron expression is null");
        return -1;
    }

    let expr_str = unsafe {
        let slice = std::slice::from_raw_parts((*expr_ptr).data.as_ptr(), (*expr_ptr).len);
        match std::str::from_utf8(slice) {
            Ok(s) => s.to_owned(),
            Err(_) => {
                throw_schedule_error("cron expression is not valid UTF-8");
                return -1;
            }
        }
    };

    let cron = match Cron::from_str(&expr_str) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("invalid cron expression '{}': {}", expr_str, e);
            throw_schedule_error(&msg);
            return -1;
        }
    };

    let next_fire = match calculate_next_fire_ms(&cron) {
        Some(ms) => ms,
        None => {
            let msg = format!("cron expression '{}' has no upcoming occurrence", expr_str);
            throw_schedule_error(&msg);
            return -1;
        }
    };

    let func: TaskFn = unsafe { std::mem::transmute(func_ptr) };
    let data = data_ptr as *mut u8;
    let size = data_size as usize;

    let id = NEXT_CRON_ID.fetch_add(1, Ordering::Relaxed);
    let entry = CronEntry {
        id,
        cron,
        func,
        data_ptr: data,
        data_size: size,
        next_fire_ms: next_fire,
    };

    let scheduler = get_cron_scheduler();
    let mut state = scheduler.state.lock().unwrap();
    state.entries.insert(id, entry);
    scheduler.condvar.notify_one();

    id as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_cancel_schedule(handle: i64) {
    let scheduler = get_cron_scheduler();
    let mut state = scheduler.state.lock().unwrap();
    if let Some(entry) = state.entries.remove(&(handle as u64)) {
        if !entry.data_ptr.is_null() && entry.data_size > 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(entry.data_size, 8);
                std::alloc::dealloc(entry.data_ptr, layout);
            }
        }
    }
    scheduler.condvar.notify_one();
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_timers_next_run(handle: i64) -> i64 {
    let scheduler = get_cron_scheduler();
    let state = scheduler.state.lock().unwrap();
    match state.entries.get(&(handle as u64)) {
        Some(entry) => {
            match calculate_next_fire_ms(&entry.cron) {
                Some(ms) => ms,
                None => 0,
            }
        }
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    static CRON_TEST_COUNTER: AtomicI64 = AtomicI64::new(0);

    extern "C" fn increment_cron_counter(_data: *mut u8) {
        CRON_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_invalid_cron_expression() {
        let result = naml_timers_schedule(
            increment_cron_counter as *const () as i64,
            std::ptr::null_mut::<u8>() as i64,
            0,
            std::ptr::null::<NamlString>() as i64,
        );
        assert_eq!(result, -1);
        naml_std_core::naml_exception_clear();
    }

    #[test]
    fn test_schedule_and_cancel() {
        CRON_TEST_COUNTER.store(0, Ordering::SeqCst);

        let expr = "* * * * * *";
        let expr_ptr = unsafe { naml_string_new(expr.as_ptr(), expr.len()) };

        let handle = naml_timers_schedule(
            increment_cron_counter as *const () as i64,
            std::ptr::null_mut::<u8>() as i64,
            0,
            expr_ptr as i64,
        );
        assert!(handle > 0, "schedule should return positive handle");

        let next = naml_timers_next_run(handle);
        assert!(next > 0, "next_run should return future timestamp");

        naml_timers_cancel_schedule(handle);

        let next_after_cancel = naml_timers_next_run(handle);
        assert_eq!(next_after_cancel, 0, "next_run after cancel should return 0");
    }
}
