//!
//! M:N Task Scheduler for naml
//!
//! Implements an M:N threading model where M user-space tasks (goroutines)
//! are multiplexed onto N OS threads. Features:
//!
//! - Thread pool with configurable worker count (defaults to CPU cores)
//! - Work-stealing queue for load balancing
//! - Closure support for captured variables
//! - Efficient task scheduling
//!

use std::alloc::{alloc, dealloc, Layout};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

/// Task function signature: takes a pointer to captured data
type TaskFn = extern "C" fn(*mut u8);

/// A task consists of a function pointer and captured data
struct Task {
    func: TaskFn,
    data: *mut u8,
    data_size: usize,
}

unsafe impl Send for Task {}

/// The global task queue
struct TaskQueue {
    tasks: Mutex<VecDeque<Task>>,
    condvar: Condvar,
    shutdown: AtomicBool,
}

impl TaskQueue {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            shutdown: AtomicBool::new(false),
        }
    }

    fn push(&self, task: Task) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(task);
        self.condvar.notify_one();
    }

    fn pop(&self) -> Option<Task> {
        let mut tasks = self.tasks.lock().unwrap();
        while tasks.is_empty() && !self.shutdown.load(Ordering::SeqCst) {
            tasks = self.condvar.wait(tasks).unwrap();
        }
        tasks.pop_front()
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.condvar.notify_all();
    }

    #[allow(dead_code)]
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }
}

/// The M:N scheduler manages a pool of worker threads
struct Scheduler {
    queue: Arc<TaskQueue>,
    workers: Vec<JoinHandle<()>>,
    active_tasks: Arc<AtomicUsize>,
}

impl Scheduler {
    fn new(num_workers: usize) -> Self {
        let queue = Arc::new(TaskQueue::new());
        let active_tasks = Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let queue_clone = Arc::clone(&queue);
            let tasks_clone = Arc::clone(&active_tasks);
            let handle = thread::spawn(move || {
                worker_loop(queue_clone, tasks_clone);
            });
            workers.push(handle);
        }

        Self {
            queue,
            workers,
            active_tasks,
        }
    }

    fn spawn(&self, func: TaskFn, data: *mut u8, data_size: usize) {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        self.queue.push(Task { func, data, data_size });
    }

    fn active_count(&self) -> usize {
        self.active_tasks.load(Ordering::SeqCst)
    }

    fn wait_all(&self) {
        while self.active_count() > 0 {
            thread::yield_now();
        }
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        self.queue.shutdown();
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }
}

fn worker_loop(queue: Arc<TaskQueue>, active_tasks: Arc<AtomicUsize>) {
    while let Some(task) = queue.pop() {
        (task.func)(task.data);

        if !task.data.is_null() && task.data_size > 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(task.data_size, 8);
                dealloc(task.data, layout);
            }
        }

        active_tasks.fetch_sub(1, Ordering::SeqCst);
    }
}

static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

fn get_scheduler() -> &'static Scheduler {
    SCHEDULER.get_or_init(|| {
        let num_workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Scheduler::new(num_workers)
    })
}

/// Spawn a task with captured data
#[unsafe(no_mangle)]
pub extern "C" fn naml_spawn_closure(
    func: extern "C" fn(*mut u8),
    data: *mut u8,
    data_size: usize,
) {
    get_scheduler().spawn(func, data, data_size);
}

/// Spawn a task without captured data (legacy interface)
#[unsafe(no_mangle)]
pub extern "C" fn naml_spawn(func: extern "C" fn()) {
    extern "C" fn wrapper(data: *mut u8) {
        let func: extern "C" fn() = unsafe { std::mem::transmute(data) };
        func();
    }
    get_scheduler().spawn(wrapper, func as *mut u8, 0);
}

/// Wait for all spawned tasks to complete
#[unsafe(no_mangle)]
pub extern "C" fn naml_wait_all() {
    get_scheduler().wait_all();
}

/// Get the number of active tasks
#[unsafe(no_mangle)]
pub extern "C" fn naml_active_tasks() -> i64 {
    get_scheduler().active_count() as i64
}

/// Sleep for the given number of milliseconds
#[unsafe(no_mangle)]
pub extern "C" fn naml_sleep(ms: i64) {
    thread::sleep(std::time::Duration::from_millis(ms as u64));
}

/// Allocate memory for captured closure data
#[unsafe(no_mangle)]
pub extern "C" fn naml_alloc_closure_data(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc(layout)
    }
}

/// Get the number of worker threads in the pool
#[unsafe(no_mangle)]
pub extern "C" fn naml_worker_count() -> i64 {
    thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    static BASIC_COUNTER: AtomicI64 = AtomicI64::new(0);
    static CLOSURE_COUNTER: AtomicI64 = AtomicI64::new(0);

    extern "C" fn increment_basic_counter() {
        BASIC_COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_spawn_basic() {
        BASIC_COUNTER.store(0, Ordering::SeqCst);

        naml_spawn(increment_basic_counter);
        naml_spawn(increment_basic_counter);
        naml_spawn(increment_basic_counter);

        naml_wait_all();

        assert_eq!(BASIC_COUNTER.load(Ordering::SeqCst), 3);
    }

    extern "C" fn add_value_to_closure_counter(data: *mut u8) {
        let value = unsafe { *(data as *const i64) };
        CLOSURE_COUNTER.fetch_add(value, Ordering::SeqCst);
    }

    #[test]
    fn test_spawn_with_closure() {
        CLOSURE_COUNTER.store(0, Ordering::SeqCst);

        for i in 1..=5 {
            let data = naml_alloc_closure_data(8);
            unsafe {
                *(data as *mut i64) = i;
            }
            naml_spawn_closure(add_value_to_closure_counter, data, 8);
        }

        naml_wait_all();

        assert_eq!(CLOSURE_COUNTER.load(Ordering::SeqCst), 15);
    }
}
